package driver

import fees.ItemKind
import fees.LateFeeService
import groovy.transform.CompileStatic

/**
 * Runs a codespec harness trace against the original Groovy fees service and prints one line per
 * command. The trace and output formats are defined in the doc of crates/harness; the Go and Rust
 * drivers print exactly the same.
 */
@CompileStatic
final class Main {

    static void main(String[] args) {
        BufferedReader input = new BufferedReader(new InputStreamReader(System.in, 'UTF-8'))
        StringBuilder out = new StringBuilder()
        LateFeeService s = null
        int n = 0
        String line = input.readLine()
        while (line != null) {
            n++
            String[] f = line.split(' ', -1)
            if (f[0] == 'new' && f.length == 1) {
                s = new LateFeeService()
                out.append('new\n')
            } else if (s == null) {
                fail(n, 'command before new')
            } else {
                run(s, n, f, line, out)
            }
            line = input.readLine()
        }
        PrintStream stdout = new PrintStream(System.out, false, 'UTF-8')
        stdout.print(out)
        stdout.flush()
    }

    private static void run(LateFeeService s, int n, String[] f, String line, StringBuilder out) {
        String cmd = f[0] + '/' + f.length
        try {
            switch (cmd) {
                case 'rate/3':
                    out.append('rate ').append(s.putRate(kind(n, f[1]), Long.parseLong(f[2]))).append('\n')
                    break
                case 'grace/2':
                    out.append('grace ').append(s.graceFor(Integer.parseInt(f[1]))).append('\n')
                    break
                case 'cap/2':
                    out.append('cap ').append(s.capAt(Long.parseLong(f[1]))).append('\n')
                    break
                case 'waive/2':
                    out.append('waive ').append(s.waive(Integer.parseInt(f[1]))).append('\n')
                    break
                case 'assess/3':
                    out.append('assess ').append(s.assess(kind(n, f[1]), Integer.parseInt(f[2]))).append('\n')
                    break
                case 'rates/1':
                    out.append('rates ').append(s.rateList()).append('\n')
                    break
                case 'stats/1':
                    out.append('stats ').append(s.stats()).append('\n')
                    break
                default:
                    fail(n, 'unknown command ' + line)
            }
        } catch (NumberFormatException e) {
            fail(n, e.message)
        }
    }

    private static ItemKind kind(int line, String s) {
        switch (s) {
            case 'book':
                return ItemKind.BOOK
            case 'disc':
                return ItemKind.DISC
            case 'tool':
                return ItemKind.TOOL
        }
        fail(line, 'unknown kind ' + s)
        return null
    }

    private static void fail(int line, String msg) {
        System.err.println('driver: trace line ' + line + ': ' + msg)
        System.exit(2)
    }
}
