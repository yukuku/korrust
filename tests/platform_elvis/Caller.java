import api.Source;
import use.ConsumerKt;
public class Caller {
    public static void main(String[] args) {
        if (!"first".equals(ConsumerKt.required("first")) || Source.reads != 0) throw new AssertionError();
        if (ConsumerKt.maybe(null) != null || ConsumerKt.field(null) != null || Source.reads != 1) throw new AssertionError();
        try { ConsumerKt.required(null); throw new AssertionError(); } catch (NullPointerException expected) { }
        Source.fallback = "other";
        if (!"other".equals(ConsumerKt.required(null)) || Source.reads != 3) throw new AssertionError();
    }
}
