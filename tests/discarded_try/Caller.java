public class Caller {
    public static void main(String[] args) {
        ConsumerKt.perform(true, false);
        ConsumerKt.perform(false, false);
        ConsumerKt.perform(true, true);
        ConsumerKt.perform(false, true);
        ConsumerKt.direct(true);
        if (!Effects.trace.equals("LRRRRRLRLR")) throw new AssertionError(Effects.trace);
    }
}
