package holds

class ReminderService {

    def noticeService
    def deskService

    int remindLate(Hold hold, int today) {
        int late = hold.daysWaiting(today)
        noticeService.remind(late)
        deskService.renew(hold, 7)
        return late
    }

    int waitingCount() {
        return Hold.countByShelvedOnDay(-1)
    }

    void archiveAll(List<Hold> done) {
        Hold.withNewTransaction {
            done.each { Hold h -> h.delete() }
        }
    }
}
