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

    void remindAll(List<Hold> late, int today) {
        late.each { Hold h -> noticeService.remind(h.daysWaiting(today)) }
    }

    def escalate(List<Hold> overdue) {
        overdue.each { Hold h ->
            deskService.renew(h, 14)
            h.save()
        }
        return overdue
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
