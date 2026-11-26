package singleton.consumer;
import singleton.library.Events;
public class Calls {
    public static void main(String[] args) {
        Events.clear();
        String order = ConsumerKt.order();
        if (!order.equals("ra:ORAM")) throw new AssertionError(order);
        Events.clear();
        String companion = ConsumerKt.companionOrder();
        if (!companion.equals("rc:CRN")) throw new AssertionError(companion);
        if (!ConsumerKt.defaults().equals("d!")) throw new AssertionError();
        if (!ConsumerKt.named().equals("dn")) throw new AssertionError();
        if (!ConsumerKt.overloaded().equals("7s")) throw new AssertionError();
        if (!ConsumerKt.inherited().equals("parent")) throw new AssertionError();
        if (!ConsumerKt.generic().equals("g")) throw new AssertionError();
        if (!ConsumerKt.nullable(null).equals("fallback")) throw new AssertionError();
        if (!ConsumerKt.nullable("present").equals("present")) throw new AssertionError();
        if (ConsumerKt.properties() != 30) throw new AssertionError();
        System.out.println("singleton-extensions-ok");
    }
}
