public class ConsumeCalls { public static void main(String[] args) { if(!ConsumerKt.consume().equals("7boundclass3"))throw new AssertionError(ConsumerKt.consume());System.out.println("OK"); } }
