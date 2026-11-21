package genericfields;
import java.util.Arrays;
public final class Driver {
    private static void equal(Object actual, Object expected) {
        if (!java.util.Objects.equals(actual, expected)) throw new AssertionError(actual + " != " + expected);
    }
    private static void raises(Class<? extends Throwable> type, Runnable action) {
        try { action.run(); } catch (Throwable error) {
            if (type.isInstance(error)) return;
            throw new AssertionError("unexpected exception", error);
        }
        throw new AssertionError("missing exception " + type.getName());
    }
    @SuppressWarnings({"rawtypes", "unchecked"})
    public static void main(String[] args) {
        Cell<String> text = new Cell<>("alpha", null, Arrays.asList("item"), new String[] {"array"});
        equal(UseKt.value(text), "alpha"); equal(UseKt.fixed(text), "alpha");
        equal(UseKt.published(text), "alpha"); equal(UseKt.required(text), "alpha");
        equal(UseKt.requiredNullable(text), "alpha");
        equal(UseKt.platform(text), "alpha"); equal(UseKt.optional(text), null);
        equal(UseKt.safe(text), "alpha"); equal(UseKt.safe(null), null); equal(UseKt.lambda(text).invoke(), "alpha");
        equal(UseKt.first(text), "item"); equal(UseKt.array(text), "array");
        equal(UseKt.inherited(new StringCell("inherited")), "inherited");
        equal(UseKt.generic(text), "alpha"); equal(UseKt.erased(text), "alpha");
        equal(UseKt.projected(text), "alpha"); equal(UseKt.starred(text), "alpha");
        equal(UseKt.consumed(text), "alpha"); equal(UseKt.plainAsAny(text), "label");
        Provider.calls = 0; equal(UseKt.indirect(), "next"); equal(Provider.calls, 1);
        Cell<Integer> number = new Cell<>(17, null, Arrays.asList(19), new Integer[] {23});
        equal(UseKt.primitive(number), 17); equal(UseKt.boxed(number), 17); equal(UseKt.intArray(number), 23);
        Cell<Long> wide = new Cell<>(4294967297L, null, null, null);
        equal(UseKt.longValue(wide), 4294967297L);
        NumberCell<Integer> bounded = new NumberCell<>(31);
        equal(UseKt.bound(bounded), 31); equal(UseKt.boundBoxed(bounded), 31);
        bounded.value = null; equal(UseKt.boundBoxed(bounded), null);
        raises(NullPointerException.class, () -> UseKt.bound(bounded));
        number.value = null; equal(UseKt.boxed(number), null); equal(UseKt.nullablePrimitive(number), null);
        raises(NullPointerException.class, () -> UseKt.primitive(number));
        text.value = null; equal(UseKt.nullable(text), null); equal(UseKt.platform(text), null);
        equal(UseKt.safe(text), null); raises(NullPointerException.class, () -> UseKt.lambda(text).invoke());
        equal(UseKt.starred(text), null); equal(UseKt.consumed(text), null);
        raises(NullPointerException.class, () -> UseKt.value(text));
        raises(NullPointerException.class, () -> UseKt.generic(text));
        Object wrong = new Object(); ((Cell) text).value = wrong;
        raises(ClassCastException.class, () -> UseKt.value(text));
        if (UseKt.erased(text) != wrong) throw new AssertionError("erasure adaptation");
        System.out.println("OK");
    }
}
