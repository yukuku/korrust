public final class ConsumeCalls {
 public static void main(String[] args) { fields.Base.counter = 5; if (ConsumerKt.consume() != 136) throw new AssertionError(); System.out.println("OK"); }
}
