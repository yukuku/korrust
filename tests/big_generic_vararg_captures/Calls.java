import capturedvarargs.*;
public class Calls { public static void main(String[] args) {
 Box<String> b=new Box<>("seed");
 if(!b.first(7,8).invoke().equals(7))throw new AssertionError("boxed capture");
 if(!b.bounded(new Named("bound")).invoke().equals("bound"))throw new AssertionError("bounded capture");
 if(!b.elements(0,"class").invoke().equals("class"))throw new AssertionError("classifier capture");
 if(b.size(1,2,3).invoke()!=3)throw new AssertionError("array capture");
 System.out.println("OK");
} }
