package ledger;

import java.util.ArrayList;
import java.util.List;

/** One account: its balance, how many postings it took and their amounts. */
public class Account {
    private final String id;
    private long balance;
    private int postings;
    private final List<Long> amounts = new ArrayList<>();

    public Account(String id) {
        this.id = id;
    }

    public String id() {
        return id;
    }

    public long balance() {
        return balance;
    }

    public int postings() {
        return postings;
    }

    /** Records one amount; a withdrawal or fee that would overdraw fails. */
    public void post(Kind kind, long amount) throws LedgerException {
        switch (kind) {
            case DEPOSIT:
                balance += amount;
                break;
            case WITHDRAWAL:
            case FEE:
                if (balance < amount) {
                    throw new InsufficientFunds();
                }
                balance -= amount;
                break;
            default:
                break;
        }
        postings++;
        amounts.add(amount);
    }

    /** The largest single amount, or 0. */
    public long largest() {
        long max = 0;
        for (int i = 0; i < amounts.size(); i++) {
            if (amounts.get(i) > max) {
                max = amounts.get(i);
            }
        }
        return max;
    }

    /** How many leading amounts it takes to reach target, or -1. */
    public int reach(long target) {
        int n = 0;
        long sum = 0;
        while (n < amounts.size()) {
            sum += amounts.get(n);
            n++;
            if (sum >= target) {
                break;
            }
        }
        if (sum < target) {
            return -1;
        }
        return n;
    }

    public String describe() {
        return String.format("%s: %d after %d postings", id, balance, postings);
    }
}
