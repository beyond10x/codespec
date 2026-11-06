package fees

enum ItemKind {
    BOOK, DISC, TOOL
}

class LateFeeService {

    static final int MAX_DAYS = 365

    Rate policy = new Rate()
    TreeMap<String, Long> rates = new TreeMap<>()
    int waivers = 0
    long collected = 0L
    int assessed = 0
    int waived = 0
    int graced = 0
    int capped = 0

    String code(ItemKind kind) {
        switch (kind) {
            case ItemKind.BOOK:
                return "book"
            case ItemKind.DISC:
                return "disc"
            case ItemKind.TOOL:
                return "tool"
        }
        return ""
    }

    int putRate(ItemKind kind, long cents) {
        rates.put(code(kind), cents)
        return rates.size()
    }

    int graceFor(int days) {
        policy.graceDays = days
        return policy.graceDays
    }

    long capAt(long cents) {
        policy.capCents = cents
        return policy.capCents
    }

    int waive(int n) {
        waivers = waivers + n
        return waivers
    }

    long assess(ItemKind kind, int daysLate) {
        assessed = assessed + 1
        if (waivers > 0) {
            waivers = waivers - 1
            waived = waived + 1
            return 0L
        }
        int days = policy.daysCharged(daysLate)
        if (days <= 0) {
            graced = graced + 1
            return 0L
        }
        if (days > MAX_DAYS) {
            days = MAX_DAYS
        }
        long rate = rates.getOrDefault(code(kind), 0L)
        if (rate == 0L) {
            rate = policy.dailyCents
        }
        long fee = 0L
        int d = 0
        while (d < days) {
            fee = fee + rate
            d = d + 1
            if (policy.capped(fee)) {
                fee = policy.capCents
                capped = capped + 1
                break
            }
        }
        collected = collected + fee
        return fee
    }

    String rateList() {
        String out = ""
        for (String k : rates.keySet()) {
            out = out + k + "=" + rates.getOrDefault(k, 0L) + ";"
        }
        return out
    }

    String stats() {
        return "assessed ${assessed} waived ${waived} graced ${graced} capped ${capped} collected ${collected}"
    }
}
