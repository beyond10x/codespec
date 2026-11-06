package lending

class Loan {

    int lentOnDay = 0 // expect: read
    int dueDay = 0 // expect: read
    int returnedDay = -1 // expect: read
    LoanStatus status = LoanStatus.OPEN // expect: read

    static belongsTo = [tool: Tool, member: Member] // expect: refused gorm

    static mapping = { // expect: refused gorm
        version false
    }

    boolean isOverdue(int today) { // expect: read
        return returnedDay < 0 && today > dueDay
    }

    int daysLate(int today) { // expect: refused
        int end = returnedDay < 0 ? today : returnedDay // expect: refused unsupported
        if (end <= dueDay) {
            return 0
        }
        return end - dueDay
    }

    int daysOverdue(int today) { // expect: read
        int end = 0
        if (returnedDay < 0) {
            end = today
        } else {
            end = returnedDay
        }
        if (end <= dueDay) {
            return 0
        }
        return end - dueDay
    }
}
