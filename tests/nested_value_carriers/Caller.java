import consumers.ConsumerKt;

public class Caller {
    public static void main(String[] args) {
        if (ConsumerKt.payload(-73588229205L) != -73588229205L || !ConsumerKt.absent() || !ConsumerKt.unsignedValue()) throw new AssertionError();
        if (ConsumerKt.deep(Long.MIN_VALUE) != Long.MIN_VALUE || ConsumerKt.present(77L) != 77L) throw new AssertionError();
    }
}
