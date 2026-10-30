package delegated;
public class Events {
    public static String trace="";
    public static int failAt;
    public static RuntimeException failure;
    public static int mark(int value){trace+=value;return value;}
    public static Object pick(Object value,int order){mark(order);if(failure!=null && failAt==order)throw failure;return value;}
}
