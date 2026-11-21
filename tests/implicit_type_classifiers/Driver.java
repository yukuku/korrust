package classifierprobe;
import java.util.Arrays;
public class Driver {
    public static void main(String[] args) {
        Catalog.Entry<String> first = new Catalog.Entry<>("a", "alpha");
        Catalog.Entry<String> second = new Catalog.Entry<>("b", "beta");
        if (!Lookup.value(first).equals("a")) throw new AssertionError();
        if (Lookup.find(Arrays.asList(first, second), "b") != second) throw new AssertionError();
        if (Lookup.find(Arrays.asList(first, second), "missing") != null) throw new AssertionError();
        if (!Lookup.marker(new Marker()) || Lookup.marker("wrong")) throw new AssertionError();
        if (!Lookup.mode(Catalog.Mode.SECOND) || Lookup.mode(new Marker())) throw new AssertionError();
        if (new Holder(Catalog.Mode.FIRST).getMode() != Catalog.Mode.FIRST) throw new AssertionError();
        Observer listener = mode -> { if (mode != Catalog.Mode.SECOND) throw new AssertionError(); };
        listener.changed(Catalog.Mode.SECOND);
        if (!aliasprobe.AliasKt.alias(first).equals("a")) throw new AssertionError();
        if (!aliasprobe.AliasKt.fullyQualified(second).equals("b")) throw new AssertionError();
        if (!Lookup.kotlinNested(new KotlinOuter.Nested()) || Lookup.kotlinNested(new Marker())) throw new AssertionError();
        if (!ConsumerKt.consume(first).equals("a")) throw new AssertionError();
        if (!ConsumerKt.consumeText(new Catalog.Entry<String>("text", "body")).equals("text")) throw new AssertionError();
        System.out.println("OK 15");
    }
}
