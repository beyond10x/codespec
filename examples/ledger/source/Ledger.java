package ledger;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Optional;
import java.util.TreeMap;

/** Accounts by id, numbered from 1 in the order they were opened, and a tally of postings per kind. */
public class Ledger {
    private final TreeMap<String, Integer> numbers = new TreeMap<>();
    private final List<Account> accounts = new ArrayList<>();
    private final HashMap<Kind, Integer> tally = new HashMap<>();
    private int count;

    /** Opens an account and returns its number. */
    public int open(String id) throws LedgerException {
        int known = numbers.getOrDefault(id, 0);
        if (known != 0) {
            throw new DuplicateAccount();
        }
        accounts.add(new Account(id));
        int n = accounts.size();
        numbers.put(id, n);
        return n;
    }

    public Optional<Account> find(String id) {
        int n = numbers.getOrDefault(id, 0);
        if (n == 0) {
            return Optional.empty();
        }
        return Optional.of(accounts.get(n - 1));
    }

    /** Posts a positive amount; a withdrawal may not overdraw, a fee may. */
    public void post(String id, Kind kind, long amount) throws LedgerException {
        if (amount <= 0) {
            throw new NonPositiveAmount();
        }
        int n = numbers.getOrDefault(id, 0);
        if (n == 0) {
            throw new UnknownAccount();
        }
        Account a = accounts.get(n - 1);
        if (kind == Kind.WITHDRAWAL && a.balance() < amount) {
            throw new InsufficientFunds();
        }
        a.post(kind, amount);
        count++;
        tally.put(kind, tally.getOrDefault(kind, 0) + 1);
    }

    /** Awards points to an account and returns its new total. */
    public int award(String id, int points) throws LedgerException {
        int n = numbers.getOrDefault(id, 0);
        if (n == 0) {
            throw new UnknownAccount();
        }
        return accounts.get(n - 1).award(points);
    }

    /** Every balance, in id order. */
    public String balances() {
        String out = "";
        for (String id : numbers.keySet()) {
            out += String.format("%s=%d;", id, accounts.get(numbers.getOrDefault(id, 0) - 1).balance());
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
