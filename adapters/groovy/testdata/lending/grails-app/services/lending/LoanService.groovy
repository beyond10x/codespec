package lending

class LoanService {

    def catalogService // expect: refused spring
    FeeService feeService // expect: refused spring

    int dueDayFor(int lentOnDay) { // expect: refused
        return DueDates.dueDay(lentOnDay) // expect: refused unsupported
    }

    void lend(Member member, Tool tool, Loan loan) { // expect: refused
        member.addToLoans(loan) // expect: refused gorm
        tool.markLent()
        loan.save() // expect: refused gorm
    }

    void forget(Loan loan) { // expect: refused
        loan.delete() // expect: refused gorm
    }

    List<Loan> loansOf(Member member) { // expect: refused
        return Loan.findAllByMember(member) // expect: refused gorm
    }

    String toolName(long toolId) { // expect: refused
        return catalogService.toolById(toolId).name // expect: refused spring
    }

    long lateFee(Loan loan, int today) { // expect: refused
        return feeService.lateFeeCents(loan.daysLate(today)) // expect: refused spring
    }
}
