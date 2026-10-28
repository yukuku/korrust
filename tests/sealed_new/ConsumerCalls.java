import sealednew.*;
public final class ConsumerCalls {
    public static void main(String[] args) {
        if (!ConsumerKt.consume(new Choice.First(7)).equals("7") || !ConsumerKt.consumeSignal(new SetValue(9)).equals("9")) throw new AssertionError();
        System.out.println("OK");
    }
}
