package holds

class RecallService extends NoticeService {

    DeskService deskService

    RecallService() {
        sent = graceDays()
    }

    void recall(Hold hold, int today) {
        deskService.renew(hold)
        remind(today - hold.placedOnDay)
        Slip slip = new Slip(hold.title)
        String line = slip.text().trim()
        hold.archive(line)
    }

    static int graceDays() {
        return 3
    }
}
