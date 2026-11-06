package lending

import grails.transaction.Transactional // expect: refused unresolved

@Transactional
class TransactionalAuditService {

    int recorded = 0 // expect: read

    int record(int entries) { // expect: read
        recorded = recorded + entries
        return recorded
    }
}
