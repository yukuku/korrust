import anonymousclasses.*;
public class Check {
    static void eq(int actual, int expected) { if (actual != expected) throw new AssertionError(actual + " != " + expected); }
    public static void main(String[] args) {
        Base base = SourceKt.create(3);
        eq(base.inherited(), 4);
        eq(((Left) base).left(), 3);
        eq(base.compute(5), 12);
        eq(((Right) base).right(), 14);
        eq(((Left) base).left(), 10);
        if (!(base instanceof Marker)) throw new AssertionError();
        if (SourceKt.create(3) == base) throw new AssertionError();
        eq(SourceKt.empty(17).compute(2), 19);
        eq(new Outer(7).make(3).compute(2), 22);
        eq(SourceKt.argumentCapture().compute(4), 6);
        if (SourceKt.marker() == SourceKt.marker()) throw new AssertionError();
        PairBase pair = SourceKt.orderedArguments();
        eq(pair.getFirst(), 1);
        eq(pair.getSecond(), 2);
        eq(pair.state(), 2);
        System.out.println("OK");
    }
}
