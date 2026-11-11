package holds

class PickupService implements Tally {

    def noticeService
    def deskService

    int tally() {
        return Hold.countByShelvedOnDay(-1)
    }

    void announce(Hold hold, int today) {
        noticeService.pickupReady(hold.daysWaiting(today))
    }

    void extendAll(List<Hold> waiting) {
        waiting.each { Hold h ->
            deskService.renew(h, 7)
            h.save()
        }
    }
}
