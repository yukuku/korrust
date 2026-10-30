public class Launch {
 public static void main(String[] args) {
  if (!objectconsumer.ConsumerKt.consume().equals("OK")) throw new AssertionError();
  System.out.println("OK");
 }
}
