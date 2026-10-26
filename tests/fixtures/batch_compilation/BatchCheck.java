public final class BatchCheck {
    static void equal(int actual, int expected) {
        if (actual != expected) throw new AssertionError(actual + " != " + expected);
    }
    public static void main(String[] args) {
        equal(batch.sample.FirstKt.inferred(), 12);
        equal(batch.sample.FirstKt.getInferredProperty(), 9);
        if (!batch.sample.FirstKt.even(8) || !batch.sample.SecondKt.odd(7)) throw new AssertionError("mutual recursion");
        if (batch.sample.FirstKt.even(7) || batch.sample.SecondKt.odd(8)) throw new AssertionError("recursion parity");
        batch.sample.Right value = new batch.sample.Right(new batch.sample.Left(null));
        if (value.getLeft().getRight() != null) throw new AssertionError("cyclic signatures");
        equal(batch.sample.FirstKt.combined(), 30);
        equal(batch.sample.FirstKt.ownPrivate(), 13);
        equal(batch.sample.FirstKt.companionValue(), 9);
        equal(batch.sample.FirstKt.publicInternal(), 17);
        equal(new batch.left.Same(4).getValue(), 4);
        equal(new batch.right.Same(9).getValue(), 9);
        System.out.println("batch-ok:30");
    }
}
