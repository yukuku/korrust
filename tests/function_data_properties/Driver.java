package functiondata;

import kotlin.jvm.functions.Function1;

public final class Driver {
    private static int checks;
    private static void check(boolean value) {
        checks++;
        if (!value) throw new AssertionError("check " + checks);
    }
    private static void rejects(Runnable body) {
        boolean rejected = false;
        try { body.run(); } catch (NullPointerException expected) { rejected = true; }
        check(rejected);
    }
    private static final class Fn implements Function1<Integer, Integer> {
        private final int offset;
        Fn(int offset) { this.offset = offset; }
        public Integer invoke(Integer value) { return value + offset; }
        public int hashCode() { return 101 + offset; }
        public String toString() { return "Fn(" + offset + ")"; }
    }
    public static void main(String[] args) {
        Fn first = new Fn(3);
        Fn second = new Fn(3);
        Function1<String, String> text = value -> value + "!";
        Handlers value = new Handlers(first, null);
        check(value.getAction() == first);
        check(value.component1() == first);
        check(value.getOptional() == null && value.component2() == null);
        check(value.getAction().invoke(9) == 12);
        check(ConsumerKt.consume(value) == 24);
        Handlers same = value.copy(first, null);
        check(same != value && same.equals(value) && value.equals(same));
        check(value.equals(value) && !value.equals(null) && !value.equals("handlers"));
        check(!value.equals(new Handlers(second, null)));
        check(value.hashCode() == 31 * first.hashCode());
        check(value.toString().equals("Handlers(action=Fn(3), optional=null)"));
        Handlers nonnull = value.copy(first, text);
        check(nonnull.getOptional() == text && nonnull.component2() == text);
        check(nonnull.getOptional().invoke("a").equals("a!"));
        check(!nonnull.equals(value) && !value.equals(nonnull));
        check(nonnull.hashCode() == 31 * first.hashCode() + text.hashCode());
        check(nonnull.toString().equals("Handlers(action=Fn(3), optional=" + text + ")"));
        rejects(() -> new Handlers(null, null));
        rejects(() -> value.copy(null, text));
        check(checks == 17);
        System.out.println("function-data-checks=" + checks);
    }
}
