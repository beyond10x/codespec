package fees

class Rate {

    long dailyCents = 10L
    int graceDays = 0
    long capCents = 0L

    int daysCharged(int daysLate) {
        int days = daysLate - graceDays
        return days
    }

    boolean capped(long fee) {
        return capCents > 0L && fee >= capCents
    }
}
