public class Caller {
    public static void main(String[] args) {
        if (!fixture.ConsumerKt.text(new fixture.Screen<String>("one")).equals("one") ||
            !fixture.ConsumerKt.echoed().equals("ok")) throw new AssertionError();
    }
}
