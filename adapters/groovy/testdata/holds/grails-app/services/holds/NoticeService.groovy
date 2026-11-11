package holds

class NoticeService {

    int sent = 0

    void pickupReady(int daysWaited) {
        sent = sent + 1
    }

    void remind(int daysLeft) {
        sent = sent + 1
    }

    int sentCount() {
        return sent
    }
}
