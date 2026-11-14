import generics.*;
public final class Calls {
    public static void main(String[] args) {
        if (!UseKt.run().equals("OKOK317OK")) throw new AssertionError(UseKt.run());
        Box<String> box = new Box<>("A\uD83D\uDE00");
        Producer<String> producer = box;
        if (!producer.get().equals(box.getValue())) throw new AssertionError();
        Sink<Object> sink = new Sink<>();
        if (sink.accept(new Object()) != 17) throw new AssertionError();
        if (!new Tagged<Marked>(new Marked("OK")).text().equals("OK")) throw new AssertionError();
        if (new Box<String>(null).get() != null) throw new AssertionError();
        try { new Holder<Named>(null); throw new AssertionError(); } catch (NullPointerException expected) {}
        BoundSink<Named> namedSink = new NamedSink();
        if (!namedSink.accept(new Named("OK")).equals("OK")) throw new AssertionError();
        try { ((BoundSink) namedSink).accept(new OtherLabel()); throw new AssertionError(); } catch (ClassCastException expected) {}
        Named name = new Named("OK");
        Holder<Named> holder = new Holder<>(name);
        Bounded<Named> bounded = holder;
        if (bounded.get() != name || holder.getValue() != name || !holder.text().equals("OK")) throw new AssertionError();
        if (new Chars<String>("A\uD83D\uDE00").size() != 3) throw new AssertionError();
        if (Holder.class.getTypeParameters()[0].getBounds()[0] != Label.class) throw new AssertionError();
        if (Bounded.class.getTypeParameters()[0].getBounds()[0] != Label.class) throw new AssertionError();
        System.out.println("OK");
    }
}
