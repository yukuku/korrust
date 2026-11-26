package receiverscopes;

public class Calls {
    public static void main(String[] args) {
        if (ScopesKt.ordered() != 712) throw new AssertionError("receiver evaluation order");
        Box box = new Box(1);
        if (!ScopesKt.same(box) || box.getValue() != 2) throw new AssertionError("apply identity");
        if (ScopesKt.alias(box) != 4) throw new AssertionError("aliased apply");
        if (ScopesKt.custom(box) != 7) throw new AssertionError("arbitrary receiver function");
        if (ScopesKt.result(box) != 12) throw new AssertionError("receiver result");
        if (ScopesKt.capture(box) != 7) throw new AssertionError("capture");
        if (!ScopesKt.nullable(null) || ScopesKt.nullable(box)) throw new AssertionError("nullable receiver");
        if (ScopesKt.nonlocal(box, true) != 47 || box.getValue() != 7) throw new AssertionError("nonlocal exit");
        if (ScopesKt.nonlocal(box, false) != 9 || box.getValue() != 8) throw new AssertionError("ordinary completion");
        if (ScopesKt.otherScopes(box) != 24) throw new AssertionError("other scopes");
        if (ScopesKt.throwing(box) != 17 || box.getValue() != 17) throw new AssertionError("exception propagation");
        receiverlib.Store target = new receiverlib.Store();
        java.util.List<String> names = java.util.Arrays.asList("a", "b");
        java.util.Map<String, Integer> counts = java.util.Collections.singletonMap("a", 2);
        java.util.List<java.util.List<String>> nested = java.util.Collections.singletonList(names);
        if (ScopesKt.populate(target, names, counts, nested) != target) throw new AssertionError("field receiver identity");
        if (target.names != names || target.counts != counts || target.nested != nested) throw new AssertionError("generic field values");
        ScopesKt.staticStore(names);
        if (receiverlib.Store.global != names) throw new AssertionError("static generic store");
        ScopesKt.clear(target);
        if (target.names != null || target.counts != null) throw new AssertionError("platform generic nulls");
        receiverlib.Store.trace = 0;
        ScopesKt.orderedStore();
        if (receiverlib.Store.trace != 12 || !receiverlib.Store.last.names.equals(java.util.Collections.singletonList("ordered"))) throw new AssertionError("field order");
        receiverlib.Store.trace = 0;
        try { ScopesKt.receiverFailure(); throw new AssertionError("missing receiver failure"); }
        catch (IllegalStateException expected) {}
        if (receiverlib.Store.trace != 1) throw new AssertionError("receiver failure evaluated value");
        receiverlib.Store.trace = 0;
        try { ScopesKt.valueFailure(); throw new AssertionError("missing value failure"); }
        catch (IllegalArgumentException expected) {}
        if (receiverlib.Store.trace != 12 || receiverlib.Store.last.names != null) throw new AssertionError("failed value stored");
        System.out.println("compiled-receivers-ok");
    }
}
