package lending

class FeeService {

    static final long DAILY_LATE_FEE_CENTS = 25L // expect: read
    static final long LOST_TOOL_FEE_CENTS = 4000L // expect: read

    long lateFeeCents(int daysLate) { // expect: refused
        if (daysLate <= 0) {
            return 0L
        }
        long fee = DAILY_LATE_FEE_CENTS * daysLate // expect: refused unsupported
        return fee
    }

    long lateFeeForDays(long lateDays) { // expect: read
        if (lateDays <= 0L) {
            return 0L
        }
        long fee = DAILY_LATE_FEE_CENTS * lateDays
        return fee
    }

    long feeForStatus(LoanStatus status, int daysLate) { // expect: read
        switch (status) { // expect: read
            case LoanStatus.OPEN:
                return lateFeeCents(daysLate)
            case LoanStatus.RETURNED:
                return 0L
            case LoanStatus.LOST:
                return LOST_TOOL_FEE_CENTS
        }
        return 0L
    }

    long sumLateFees(List<Loan> loans, int today) { // expect: read
        long total = 0L
        for (Loan loan : loans) { // expect: read
            total = total + lateFeeCents(loan.daysLate(today))
        }
        return total
    }

    int countOverdue(List<Loan> loans, int today, int limit) { // expect: read
        int count = 0
        int i = 0
        while (i < loans.size()) { // expect: read
            if (count >= limit) {
                break
            }
            if (loans.get(i).isOverdue(today)) {
                count = count + 1
            }
            i = i + 1
        }
        return count
    }

    String receipt(String memberName, String toolName) { // expect: read
        String line = "Member " + memberName + " borrowed " + toolName
        return line
    }

    String splitFeeText(int cents, int parts) { // expect: refused
        return "share " + (cents / parts) // expect: refused groovy-semantics
    }

    String overdueLine(String memberName, boolean overdue) { // expect: refused
        return "${memberName} overdue: ${overdue}" // expect: refused groovy-semantics
    }

    String loansText(List<Loan> loans) { // expect: refused
        if (loans) { // expect: refused groovy-semantics
            return "has loans"
        }
        return "no loans"
    }
}
