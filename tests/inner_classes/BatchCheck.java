import innerprobe.Outer;
import innerconsumer.BatchConsumerKt;
public final class BatchCheck {
    public static void main(String[] args) {
        Outer outer = new Outer(11);
        Outer.Inner inner = BatchConsumerKt.batchMake(outer);
        if (BatchConsumerKt.batchRead(inner) != 13 || inner.outer() != outer) throw new AssertionError();
        outer.setValue(20);
        if (BatchConsumerKt.batchRead(inner) != 22) throw new AssertionError();
    }
}
