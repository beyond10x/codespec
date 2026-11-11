package holds

class Slip {

    String title

    Slip(String title) {
        this.title = title
    }

    String text() {
        return 'Hold: ' + title
    }
}
