public class OracleCalls {
 public static void main(String[] args) {
  String result=receiveroracle.OracleReceiverKt.observation();
  if(!result.equals("RV14")) throw new AssertionError(result);
  System.out.println(result);
 }
}
