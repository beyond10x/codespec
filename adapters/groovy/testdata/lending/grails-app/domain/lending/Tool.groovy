package lending

class Tool {

    String name // expect: read
    String shelf // expect: read
    int timesLent = 0 // expect: read
    long dailyFeeCents = 0L // expect: read

    static hasMany = [loans: Loan] // expect: refused gorm

    static constraints = { // expect: refused gorm
        name blank: false, unique: true
        shelf nullable: true
    }

    static mapping = { // expect: refused gorm
        table 'tool'
        loans sort: 'lentOnDay'
    }

    long feeFor(int days) { // expect: refused
        long total = dailyFeeCents * days // expect: refused unsupported
        return total
    }

    long feeForSpan(long spanDays) { // expect: read
        long total = dailyFeeCents * spanDays
        return total
    }

    void markLent() { // expect: read
        timesLent = timesLent + 1
    }

    String shelfTag(String prefix, String code) { // expect: read
        return prefix + "/" + code
    }

    def methodMissing(String method, args) { // expect: refused dynamic
        return null
    }
}
