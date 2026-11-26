import storage.ConsumerKt;
import storage.Holder;

public class Caller {
    public static void main(String[] args) {
        Holder item = new Holder(8);
        if (ConsumerKt.read() != 4 || ConsumerKt.store(item) != item || Holder.last != item)
            throw new AssertionError();
        if (ConsumerKt.store(null) != null || !ConsumerKt.named().equals("named"))
            throw new AssertionError();
    }
}
