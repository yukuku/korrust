package compatibility.consumer;

public class Calls {
    public static void main(String[] arguments) {
        if (ConsumerKt.value() != 103) throw new AssertionError(ConsumerKt.value());
        if (!ConsumerKt.label("meta").equals("meta")) throw new AssertionError();
        if (!Tagged.class.getAnnotation(compatibility.library.Marker.class).label().equals("older")) {
            throw new AssertionError();
        }
        System.out.println("OK");
    }
}
