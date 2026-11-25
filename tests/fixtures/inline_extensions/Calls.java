import usage.Effects;
import usage.UseKt;
public class Calls {
    static void check(boolean condition) { if (!condition) throw new AssertionError(); }
    public static void main(String[] args) {
        Effects effects = new Effects();
        check(UseKt.use(effects) == 9 && effects.getCalls() == 1);
        check(UseKt.wide() == 10000000007L);
        check(UseKt.scalar() == 3 && UseKt.nested() == 6);
        check(UseKt.nullable(null) == 17 && UseKt.nullable("wide") == 4);
        check(UseKt.retained() == 15L && UseKt.escape() == 10 && UseKt.captures(5) == 8);
        check(UseKt.reified("text") && !UseKt.reified(12) && !UseKt.reified(null));
        System.out.println("OK");
    }
}
