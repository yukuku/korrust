import sealednew.*;
public final class Calls {
    static final class Unknown implements Signal {}
    public static void main(String[] args) {
        if (!UseKt.choose(new Choice.First(7)).equals("7") || !UseKt.choose(new Choice.Second("OK")).equals("OK")) throw new AssertionError();
        if (!UseKt.signal(new Started()).equals("start") || !UseKt.signal(new Stopped()).equals("stop") || !UseKt.signal(null).equals("null")) throw new AssertionError();
        try { UseKt.signal(new Unknown()); throw new AssertionError(); } catch (kotlin.NoWhenBranchMatchedException expected) {}
        try { UseKt.choose(null); throw new AssertionError(); } catch (NullPointerException expected) {}
        if (!UseKt.tree(new Left()).equals("branch") || !UseKt.tree(new Right()).equals("branch") || !UseKt.tree(new Tip()).equals("tip")) throw new AssertionError();
        if (!UseKt.negative(new Choice.First(1)).equals("first") || !UseKt.negative(new Choice.Second("x")).equals("second")) throw new AssertionError();
        if (UseKt.action(Action.Begin.INSTANCE) != 1 || UseKt.action(Action.End.INSTANCE) != 2 || UseKt.action(Phase.UP) != 3 || UseKt.action(Phase.DOWN) != 4) throw new AssertionError();
        if (UseKt.bool(true) != 1 || UseKt.bool(false) != 0 || UseKt.bool(null) != -1) throw new AssertionError();
        if (UseKt.statement(new Choice.First(7)) != 7 || UseKt.assigned(new Choice.Second("abc")) != 3) throw new AssertionError();
        try { UseKt.throwing(new Choice.First(1)); throw new AssertionError(); } catch (IllegalArgumentException expected) { if (!"first".equals(expected.getMessage())) throw new AssertionError(); }
        if (!UseKt.throwing(new Choice.Second("OK")).equals("OK")) throw new AssertionError();
        Payload payload = new Payload("OK", 0x100000001L);
        if (!payload.getLabel().equals("OK") || payload.getCount() != 0x100000001L) throw new AssertionError();
        if (!UseKt.outcome(new Outcome.Ready("OK")).equals("OK") || !UseKt.outcome(new Outcome.Failed(7)).equals("7")) throw new AssertionError();
        if (UseKt.allSignal(new SetValue(9)) != 9 || !new SetValue(9).equals(new SetValue(9))) throw new AssertionError();
        if (UseKt.nullableResult(new Choice.First(1)) != null || !UseKt.nullableResult(new Choice.Second("OK")).equals("OK")) throw new AssertionError();
        if (UseKt.effects(new Choice.First(17)) != 17 || Probe.observations != 1 || Probe.marked != 17) throw new AssertionError();
        if (UseKt.effects(new Choice.Second("abcd")) != 4 || Probe.observations != 2 || Probe.marked != 4) throw new AssertionError();
        if (UseKt.ordinaryStatement(0) != 0 || UseKt.ordinaryStatement(1) != 7 || UseKt.subjectlessStatement(false) != 0 || UseKt.subjectlessStatement(true) != 9) throw new AssertionError();
        if (UseKt.nullableTypeTest(null) != 1 || UseKt.nullableTypeTest(new Started()) != 1) throw new AssertionError();
        Operation increment = new Increment(3);
        if (increment.apply(4) != 7 || UseKt.dispatch(increment, 5) != 8 || UseKt.dispatch(new Scale(4), 3) != 12 || UseKt.fromBase(new Concrete()) != 7) throw new AssertionError();
        UseKt.markSignal(new SetValue(19));
        if (Probe.marked != 19) throw new AssertionError();
        try { UseKt.markSignal(new Unknown()); throw new AssertionError(); } catch (kotlin.NoWhenBranchMatchedException expected) {}
        System.out.println("OK");
    }
}
