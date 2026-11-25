import overrideoperators.*;
public class EnumCalls { public static void main(String[] args) {
 if(Choice.FIRST.compareTo(Choice.SECOND)!=-1 || ((Comparable<Choice>)Choice.FIRST).compareTo(Choice.SECOND)!=-1 || EnumsKt.enumOrder()!=-1)throw new AssertionError("inherited enum comparison");
 if(Choice.FIRST.getCode()!=7||Choice.SECOND.getCode()!=7||!Choice.FIRST.toString().equals("first"))throw new AssertionError("enum defaults");
 System.out.println("OK");
} }
