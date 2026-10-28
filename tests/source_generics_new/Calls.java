import sourcegeneric.*;
public final class Calls {
    public static void main(String[] args) {
        if (!UseKt.box().equals("OK")) throw new AssertionError();
        Box<String> box = new Box<>("O");
        if (!box.identity("OK").equals("OK")) throw new AssertionError();
        Echo<String> echo = new StringEcho();
        if (!echo.echo("O").equals("OK")) throw new AssertionError();
        try { ((Echo)echo).echo(1); throw new AssertionError(); } catch (ClassCastException expected) {}
        try { ((Echo)echo).echo(null); throw new AssertionError(); } catch (NullPointerException expected) {}
        if (!UseKt.inherited().equals("OK") || UseKt.intEcho() != 5 || !UseKt.genericEcho().equals("OK") || UseKt.nullableBox() != null || UseKt.numberBox() != 7) throw new AssertionError();
        if (UseKt.tailWhen(2) != 3 || UseKt.tailWhen(1) != 2 || UseKt.tailWhen(0) != 0) throw new AssertionError();
        Relay<Integer> numeric = new IntEcho();
        if (numeric.echo(8) != 9) throw new AssertionError();
        Parent<String> parent = new StringParent("start");
        if (!parent.getOriginal().equals("start") || !parent.replace("O").equals("OK")) throw new AssertionError();
        try { ((Parent) parent).replace(3); throw new AssertionError(); } catch (ClassCastException expected) {}
        Bounded<Integer> bounded = new Bounded<>(7);
        if (bounded.getValue() != 7) throw new AssertionError();
        Box<Integer> integer = new Box<>(4);
        integer.setValue(9);
        if (integer.getValue() != 9) throw new AssertionError();
        Echo<Long> wide = new LongEcho();
        if (wide.echo(0x100000000L) != 0x100000001L) throw new AssertionError();
        if (new BooleanEcho().echo(true)) throw new AssertionError();
        if (new CharEcho().echo('\ud800') != '\ud800') throw new AssertionError();
        if (!((Echo) numeric).echo(Double.valueOf(3.9)).equals(4)) throw new AssertionError();
        try { ((Echo) numeric).echo(null); throw new AssertionError(); } catch (NullPointerException expected) {}
        Probe.calls = 0;
        if (UseKt.boxedEffects() != 3 || Probe.calls != 2 || !UseKt.inheritedProperty().equals("OK")) throw new AssertionError();
        Slot<String> stringSlot = new StringSlot("O");
        stringSlot.setValue("OK");
        if (!stringSlot.getValue().equals("OK")) throw new AssertionError();
        Slot<Integer> intSlot = new IntSlot(3);
        intSlot.setValue(5);
        if (intSlot.getValue() != 5) throw new AssertionError();
        try { ((Slot) stringSlot).setValue(3); throw new AssertionError(); } catch (ClassCastException expected) {}
        System.out.println("OK");
    }
}
