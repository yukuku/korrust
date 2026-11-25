public class Calls {
    static void check(boolean value) { if (!value) throw new AssertionError(); }
    public static void main(String[] args) {
        int[] log = {0};
        check(ProbeKt.normal(log) == 11 && log[0] == 1);
        log[0] = 0;
        check(ProbeKt.thrown(log) == 17 && log[0] == 23);
        log[0] = 0;
        check(ProbeKt.nonlocal(log) == 10000000001L && log[0] == 1);
        log[0] = 0;
        check(ProbeKt.nestedReturn(log) == 23 && log[0] == 45);
        log[0] = 0;
        check(ProbeKt.sourceFinally(log) == 31 && log[0] == 451);
        log[0] = 0;
        check(ProbeKt.finalizerThrow(log) == 6 && log[0] == 6);
        log[0] = 0;
        check(ProbeKt.finalizerLambda(log) == 51 && log[0] == 27);
        log[0] = 0;
        check(ProbeKt.overrideReturn(log) == 61 && log[0] == 0);
        log[0] = 0;
        check(ProbeKt.localReturn(log) == 71 && log[0] == 1);
        log[0] = 0;
        check(ProbeKt.reified("yes", log) && log[0] == 9);
        log[0] = 0;
        check(ProbeKt.reified(null, log) && log[0] == 9);
        log[0] = 0;
        check(!ProbeKt.reified(3, log) && log[0] == 9);
        System.out.println("12 checks");
    }
}
