import { parse, stringify } from 'yaml'
import fs from 'fs/promises'

const getIds = (yaml) => {
    //public/assigned_numbers/company_identifiers/company_identifiers.yaml
    fs.readFile(yaml, 'utf-8')
        .then((data) => {
            const parsed = parse(data)
            const ids = parsed.company_identifiers
            console.log(ids)
            return ids
        })
        .catch((err) => {
            console.log(err)
        })  


}
getIds('public/assigned_numbers/company_identifiers/company_identifiers.yaml')