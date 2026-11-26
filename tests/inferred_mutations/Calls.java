public final class Calls {
    public static void main(String[] args) {
        InferredCounter counter = new InferredCounter(5);
        if (counter.update(true) != 7 || !counter.isStored() || counter.getSetterCalls() != 1) throw new AssertionError();
        if (counter.update(false) != 6 || counter.isStored() || counter.getSetterCalls() != 2) throw new AssertionError();
        if (counter.getCurrent() != 6) throw new AssertionError();
        System.out.println("OK");
    }
}
