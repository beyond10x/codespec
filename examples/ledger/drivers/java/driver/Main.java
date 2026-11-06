package driver;

import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStreamReader;
import java.io.PrintStream;
import java.nio.charset.StandardCharsets;
import java.util.Locale;
import java.util.Optional;
import ledger.Account;
import ledger.Kind;
import ledger.Ledger;
import ledger.LedgerException;

/**
 * Runs a codespec harness trace against the original Java ledger package and prints one line per
 * command. The trace and output formats are defined in the doc of crates/harness; the Go and Rust
 * drivers print exactly the same. The source is read under Locale.ROOT, as the reader assumes.
 */
public final class Main {
    private Main() {}

    public static void main(String[] args) throws IOException {
        Locale.setDefault(Locale.ROOT);
        BufferedReader in = new BufferedReader(new InputStreamReader(System.in, StandardCharsets.UTF_8));
        StringBuilder out = new StringBuilder();
        Ledger l = null;
        int n = 0;
        for (String line = in.readLine(); line != null; line = in.readLine()) {
            n++;
            String[] f = line.split(" ", -1);
            if (f[0].equals("new") && f.length == 1) {
                l = new Ledger();
                out.append("new\n");
                continue;
            }
            if (l == null) {
                fail(n, "command before new");
            }
            String cmd = f[0] + "/" + f.length;
            try {
                switch (cmd) {
                    case "open/2":
                        int opened = l.open(f[1]);
                        out.append("open ok ").append(opened).append('\n');
                        break;
                    case "post/4":
                        l.post(f[1], kind(n, f[2]), Long.parseLong(f[3]));
                        out.append("post ok\n");
                        break;
                    case "award/3":
                        int total = l.award(f[1], Integer.parseInt(f[2]));
                        out.append("award ok ").append(total).append('\n');
                        break;
                    case "find/2": {
                        Optional<Account> a = l.find(f[1]);
                        out.append(a.isPresent() ? "find found " + a.get().describe() : "find missing").append('\n');
                        break;
                    }
                    case "largest/2": {
                        Optional<Account> a = l.find(f[1]);
                        out.append(a.isPresent() ? "largest " + a.get().largest() : "largest missing").append('\n');
                        break;
                    }
                    case "reach/3": {
                        Optional<Account> a = l.find(f[1]);
                        long target = Long.parseLong(f[2]);
                        out.append(a.isPresent() ? "reach " + a.get().reach(target) : "reach missing").append('\n');
                        break;
                    }
                    case "balances/1":
                        out.append("balances ").append(l.balances()).append('\n');
                        break;
                    case "tallied/1":
                        out.append("tallied ").append(l.tallied()).append('\n');
                        break;
                    case "count/1":
                        out.append("count ").append(l.count()).append('\n');
                        break;
                    default:
                        fail(n, "unknown command " + line);
                }
            } catch (LedgerException e) {
                out.append(f[0]).append(" err ").append(e.getMessage()).append('\n');
            } catch (NumberFormatException e) {
                fail(n, e.getMessage());
            }
        }
        PrintStream stdout = new PrintStream(System.out, false, "UTF-8");
        stdout.print(out);
        stdout.flush();
    }

    private static Kind kind(int line, String s) {
        switch (s) {
            case "deposit":
                return Kind.DEPOSIT;
            case "withdrawal":
                return Kind.WITHDRAWAL;
            case "fee":
                return Kind.FEE;
            default:
                fail(line, "unknown kind " + s);
                return null;
        }
    }

    private static void fail(int line, String msg) {
        System.err.println("driver: trace line " + line + ": " + msg);
        System.exit(2);
    }
}
