package fieldreader;
import fieldapi.Capsule;
public final class ReaderCalls {
    public static void main(String[] args) {
        Capsule value = new Capsule("reader", 4, "normal");
        if (!"reader5korrustnormal".equals(LibraryReaderKt.update(value))) throw new AssertionError();
        if (value.count != 5 || !value.label.equals("korrust")) throw new AssertionError();
        System.out.println("OK");
    }
}
