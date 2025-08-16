const truncateId = (str) => {
    if (str) {
        if (str.length > 12) {
            return `${str.slice(0, 4)}...${str.slice(-4)}`
        }
        return str
    }
}
export default truncateId