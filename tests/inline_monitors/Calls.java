public final class Calls {
    private static void check(boolean condition) { if (!condition) throw new AssertionError(); }
    public static void main(String[] args) {
        Object outer = new Object();
        Object inner = new Object();
        check(MonitorKt.critical(outer));
        check(!Thread.holdsLock(outer));
        check(MonitorKt.nested(outer, inner));
        check(!Thread.holdsLock(outer) && !Thread.holdsLock(inner));
        check(MonitorKt.nested(outer, outer));
        check(!Thread.holdsLock(outer));
        check(MonitorKt.call(outer, () -> { check(Thread.holdsLock(outer)); return 42; }) == 42);
        check(!Thread.holdsLock(outer));
        try { MonitorKt.abrupt(outer); throw new AssertionError(); }
        catch (IllegalStateException expected) { check(expected.getMessage().equals("failure")); }
        check(!Thread.holdsLock(outer));
        try { MonitorKt.call(outer, () -> { throw new IllegalArgumentException("callback"); }); throw new AssertionError(); }
        catch (IllegalArgumentException expected) { check(expected.getMessage().equals("callback")); }
        check(!Thread.holdsLock(outer));
        check(MonitorKt.early(outer) == 7);
        check(!Thread.holdsLock(outer));
        check(MonitorKt.tryOutside(outer) == 9);
        check(!Thread.holdsLock(outer));
        System.out.println("OK");
    }
}
