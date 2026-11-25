import inheriteddefaults.*;
public class Calls { public static void main(String[] args) {
 if(!InheritedKt.direct().equals("child:defaultkeptchildgrand:default"))throw new AssertionError(InheritedKt.direct());
 if(!InheritedKt.throughBase().equals("child:defaultkeptchildnumber:defaultkept19"))throw new AssertionError(InheritedKt.throughBase());
 if(!InheritedKt.directNumbers().equals("number:defaultkept19"))throw new AssertionError(InheritedKt.directNumbers());
 if(!new TextChild().choose(1,"explicit").equals("child:explicit"))throw new AssertionError();
 if(!new TextChild().own().equals("child:default"))throw new AssertionError("implicit receiver");
 System.out.println("OK");
} }
