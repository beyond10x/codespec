package lending

class Member {

    String fullName // expect: read
    String email // expect: read
    int activeLoans = 0 // expect: read
    long balanceCents = 0L // expect: read

    static hasMany = [loans: Loan] // expect: refused gorm

    static constraints = { // expect: refused gorm
        fullName blank: false
        email email: true
    }

    boolean canBorrow(int limit) { // expect: read
        return activeLoans < limit && balanceCents <= 0L
    }

    String greeting(String first) { // expect: read
        return "Hello, " + first
    }
}
