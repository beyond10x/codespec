package lending

class DueDates {

    static final int LOAN_DAYS = 14

    static int dueDay(int lentOnDay) {
        return lentOnDay + LOAN_DAYS
    }
}
