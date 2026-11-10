//! Direct CoreBluetooth scanner for macOS.
//!
//! Unlike btleplug, this sees every advertisement callback exactly once, and
//! it can read the "connectable" flag from the advertisement data.

use std::ffi::{c_char, c_void, CString};
use std::sync::mpsc::{channel as std_channel, Sender as StdSender};

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{define_class, msg_send, AnyThread, DefinedClass};
use objc2_core_bluetooth::{
    CBAdvertisementDataIsConnectable, CBAdvertisementDataLocalNameKey,
    CBAdvertisementDataManufacturerDataKey, CBAdvertisementDataServiceDataKey,
    CBAdvertisementDataServiceUUIDsKey, CBAdvertisementDataTxPowerLevelKey, CBCentralManager,
    CBCentralManagerDelegate, CBCentralManagerScanOptionAllowDuplicatesKey, CBManager,
    CBManagerAuthorization, CBManagerState, CBPeripheral, CBUUID,
};
use objc2_foundation::{
    NSArray, NSData, NSDictionary, NSMutableDictionary, NSNumber, NSObject, NSObjectProtocol,
    NSString,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::device::{hex, Mfr, Sighting, SvcData};
use crate::ids;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NativeState {
    PoweredOn,
    PoweredOff,
    Unauthorized,
    Unsupported,
    Other,
}

pub enum NativeEvent {
    State(NativeState),
    Advert(Sighting),
}

unsafe extern "C" {
    fn dispatch_queue_create(label: *const c_char, attr: *const c_void) -> *mut c_void;
}

fn cbuuid_string(uuid: &CBUUID) -> String {
    ids::normalize_uuid(&unsafe { uuid.UUIDString() }.to_string())
}

fn start_scan(central: &CBCentralManager) {
    // Without this, CoreBluetooth reports a device once and then goes quiet.
    let options: Retained<NSMutableDictionary<NSString, AnyObject>> = NSMutableDictionary::new();
    options.insert(
        unsafe { CBCentralManagerScanOptionAllowDuplicatesKey },
        &*Retained::into_super(Retained::into_super(NSNumber::new_bool(true))),
    );
    unsafe {
        central.scanForPeripheralsWithServices_options(
            None,
            Some(&*Retained::into_super(options)),
        )
    };
}

fn sighting_from_advertisement(
    peripheral: &CBPeripheral,
    adv: &NSDictionary<NSString, AnyObject>,
    rssi: &NSNumber,
) -> Sighting {
    let id = unsafe { peripheral.identifier() }
        .UUIDString()
        .to_string()
        .to_lowercase();

    let local_name = adv
        .objectForKey(unsafe { CBAdvertisementDataLocalNameKey })
        .and_then(|v| v.downcast::<NSString>().ok())
        .map(|s| s.to_string());
    // The name CoreBluetooth has cached for the peripheral; it can differ
    // from the name in this advertisement.
    let advertisement_name = unsafe { peripheral.name() }.map(|s| s.to_string());

    let mut manufacturers = Vec::new();
    if let Some(data) = adv
        .objectForKey(unsafe { CBAdvertisementDataManufacturerDataKey })
        .and_then(|v| v.downcast::<NSData>().ok())
    {
        let bytes = data.to_vec();
        if bytes.len() >= 2 {
            manufacturers.push(Mfr {
                company_id: u16::from_le_bytes([bytes[0], bytes[1]]),
                data_hex: hex(&bytes[2..]),
            });
        }
    }

    let mut service_data = Vec::new();
    if let Some(dict) = adv.objectForKey(unsafe { CBAdvertisementDataServiceDataKey }) {
        // SAFETY: documented as NSDictionary<CBUUID, NSData>.
        let dict: Retained<NSDictionary<CBUUID, NSData>> = unsafe { Retained::cast_unchecked(dict) };
        for key in dict.keys() {
            if let Some(data) = dict.objectForKey(&key) {
                service_data.push(SvcData {
                    uuid: cbuuid_string(&key),
                    data_hex: hex(&data.to_vec()),
                });
            }
        }
    }

    let mut services = Vec::new();
    if let Some(list) = adv.objectForKey(unsafe { CBAdvertisementDataServiceUUIDsKey }) {
        // SAFETY: documented as NSArray<CBUUID>.
        let list: Retained<NSArray<CBUUID>> = unsafe { Retained::cast_unchecked(list) };
        for uuid in list {
            services.push(cbuuid_string(&uuid));
        }
    }

    let tx_power = adv
        .objectForKey(unsafe { CBAdvertisementDataTxPowerLevelKey })
        .and_then(|v| v.downcast::<NSNumber>().ok())
        .map(|n| n.as_i16());
    let connectable = adv
        .objectForKey(unsafe { CBAdvertisementDataIsConnectable })
        .and_then(|v| v.downcast::<NSNumber>().ok())
        .map(|n| n.as_bool());

    // 127 is CoreBluetooth's "RSSI not available".
    let rssi = rssi.as_i16();
    let rssi = (rssi != 127).then_some(rssi);

    Sighting {
        id,
        local_name,
        advertisement_name,
        manufacturers,
        service_data,
        services,
        rssi,
        tx_power,
        appearance: None,
        address: None,
        address_type: None,
        connectable,
    }
}

define_class!(
    #[derive(Debug)]
    #[unsafe(super(NSObject))]
    #[thread_kind = AnyThread]
    #[ivars = UnboundedSender<NativeEvent>]
    struct Delegate;

    unsafe impl NSObjectProtocol for Delegate {}

    unsafe impl CBCentralManagerDelegate for Delegate {
        #[unsafe(method(centralManagerDidUpdateState:))]
        fn did_update_state(&self, central: &CBCentralManager) {
            let state = match unsafe { central.state() } {
                CBManagerState::PoweredOn => {
                    start_scan(central);
                    NativeState::PoweredOn
                }
                CBManagerState::PoweredOff => NativeState::PoweredOff,
                CBManagerState::Unauthorized => NativeState::Unauthorized,
                CBManagerState::Unsupported => NativeState::Unsupported,
                _ => NativeState::Other,
            };
            let _ = self.ivars().send(NativeEvent::State(state));
        }

        #[unsafe(method(centralManager:didDiscoverPeripheral:advertisementData:RSSI:))]
        fn did_discover(
            &self,
            _central: &CBCentralManager,
            peripheral: &CBPeripheral,
            adv: &NSDictionary<NSString, AnyObject>,
            rssi: &NSNumber,
        ) {
            let sighting = sighting_from_advertisement(peripheral, adv, rssi);
            let _ = self.ivars().send(NativeEvent::Advert(sighting));
        }
    }
);

impl Delegate {
    fn new(sender: UnboundedSender<NativeEvent>) -> Retained<Self> {
        let this = Delegate::alloc().set_ivars(sender);
        unsafe { msg_send![super(this), init] }
    }
}

/// Keeps the CoreBluetooth manager alive; scanning stops when it is dropped.
pub struct NativeScanner {
    _stop: StdSender<()>,
}

impl NativeScanner {
    pub fn start(events: UnboundedSender<NativeEvent>) -> Result<NativeScanner, String> {
        let authorization = unsafe { CBManager::authorization_class() };
        if authorization != CBManagerAuthorization::AllowedAlways
            && authorization != CBManagerAuthorization::NotDetermined
        {
            return Err("Bluetooth permission denied. Allow it in System Settings > Privacy & Security > Bluetooth.".into());
        }

        let (stop_tx, stop_rx) = std_channel::<()>();
        std::thread::Builder::new()
            .name("corebluetooth".into())
            .spawn(move || {
                // Objective-C objects are created and dropped on this thread;
                // callbacks arrive on the serial queue below.
                let delegate = Delegate::new(events);
                let label = CString::new("ble-radar.scan").unwrap();
                let queue = unsafe { dispatch_queue_create(label.as_ptr(), std::ptr::null()) };
                let queue: *mut AnyObject = queue.cast();
                let manager: Retained<CBCentralManager> = unsafe {
                    msg_send![CBCentralManager::alloc(), initWithDelegate: &*delegate, queue: queue]
                };
                // Blocks until the NativeScanner is dropped.
                let _ = stop_rx.recv();
                unsafe { manager.stopScan() };
                drop(manager);
                drop(delegate);
            })
            .map_err(|e| format!("Could not start the scanner thread: {e}"))?;

        Ok(NativeScanner { _stop: stop_tx })
    }
}
