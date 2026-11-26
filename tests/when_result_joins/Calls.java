package whenjoinscheck;
import whenjoins.JoinsKt;
import whenjoins.Probe;
public class Calls {
    public static void main(String[] args) {
        if (JoinsKt.nullable(0).getCode() != 10 || JoinsKt.nullable(1).getCode() != 20 || JoinsKt.nullable(9) != null) throw new AssertionError("nullable else");
        if (JoinsKt.nullFirst(0) != null || JoinsKt.nullFirst(1).getCode() != 30 || JoinsKt.nullFirst(9).getCode() != 40) throw new AssertionError("nullable branch");
        if (JoinsKt.nullBranch(true) != null || !JoinsKt.nullBranch(false).equals("value")) throw new AssertionError("null-first reference");
        if (JoinsKt.exhaustive(true).getCode() != 50 || JoinsKt.exhaustive(false) != null) throw new AssertionError("exhaustive");
        if (!JoinsKt.throwing(true).equals("normal")) throw new AssertionError();
        try { JoinsKt.throwing(false); throw new AssertionError(); } catch (IllegalArgumentException expected) { if (!expected.getMessage().equals("abrupt")) throw new AssertionError(); }
        if (!JoinsKt.returning(true).equals("normal") || !JoinsKt.returning(false).equals("returned")) throw new AssertionError("return branch");
        if (JoinsKt.boxed(true).intValue() != 17 || JoinsKt.boxed(false) != null) throw new AssertionError("boxed primitive");
        if (JoinsKt.wide(true).longValue() != 4294967298L || JoinsKt.wide(false) != null) throw new AssertionError("wide boxed primitive");
        Probe probe = new Probe();
        if (probe.select(1).getCode() != 70 || probe.getHits() != 1) throw new AssertionError("selected branch");
        if (probe.select(9) != null || probe.getHits() != 1) throw new AssertionError("unselected branch");
        System.out.println("when-joins-ok");
    }
}
