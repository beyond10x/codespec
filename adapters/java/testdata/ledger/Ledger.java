package ledger;

import java.util.HashMap;
import java.util.Optional;
import java.util.TreeMap;

/** Accounts by id, and a tally of postings per kind. */
public class Ledger {
    static final long MAX_AMOUNT = 1000000L;

    private final TreeMap<String, Account> accounts = new TreeMap<>();
    private final HashMap<Kind, Integer> tally = new HashMap<>();
    private int count;

    public Account open(String id) {
        Account a = accounts.get(id);
        if (a == null) {
            a = new Account(id);
            accounts.put(id, a);
        }
        return a;
    }

    public Optional<Account> find(String id) {
        Account a = accounts.get(id);
        if (a == null) {
            return Optional.empty();
        }
        return Optional.of(a);
    }

    public void post(String id, Kind kind, long amount) throws LedgerException {
        if (amount > MAX_AMOUNT) {
            throw new InsufficientFunds();
        }
        Account a = accounts.get(id);
        if (a == null) {
            throw new UnknownAccount();
        }
        a.post(kind, amount);
        count++;
        tally.put(kind, tally.getOrDefault(kind, 0) + 1);
    }

    /** Every balance, in id order. */
    public String balances() {
        String out = "";
        for (String id : accounts.keySet()) {
            out += String.format("%s=%d;", id, accounts.get(id).balance());
        }
        return out;
    }

    /** The postings tallied over every kind; the order of the kinds does not change a sum. */
    public int tallied() {
        int total = 0;
        for (Kind k : tally.keySet()) {
            total += tally.getOrDefault(k, 0);
        }
        return total;
    }

    public int count() {
        return count;
    }
}
