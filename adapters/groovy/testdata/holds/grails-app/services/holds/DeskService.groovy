package holds

class DeskService {

    NoticeService noticeService

    int checkIn(Hold hold, int today) {
        int waited = hold.daysWaiting(today)
        shelve(hold, today)
        return waited
    }

    def shelve(Hold hold, int today) {
        hold.shelvedOnDay = today
        noticeService.pickupReady(hold.daysWaiting(today))
        return hold
    }

    void renew(Hold hold) {
        hold.placedOnDay = hold.placedOnDay + 7
    }

    void renew(Hold hold, int days) {
        hold.placedOnDay = hold.placedOnDay + days
        noticeService.remind(days)
    }

    int renewAll(List<Hold> waiting, int days) {
        waiting.each { Hold h -> renew(h, days) }
        return waiting.size()
    }
}
