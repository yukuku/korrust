public final class ConsumeCalls { public static void main(String[] args) { if (ConsumerKt.consumeDiscard()!=17) throw new AssertionError(); System.out.println("OK"); } }
