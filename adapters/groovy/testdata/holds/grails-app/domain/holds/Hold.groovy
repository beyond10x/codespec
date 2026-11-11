package holds

class Hold {

    String title = ''
    int placedOnDay = 0
    int shelvedOnDay = -1

    static constraints = {
        title blank: false
    }

    int daysWaiting(int today) {
        return today - placedOnDay
    }

    boolean isShelved() {
        return shelvedOnDay >= 0
    }

    void shelveOn(int day) {
        shelvedOnDay = day
        save()
    }
}
