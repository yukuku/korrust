package library;
public class Warehouse {
    public static class Tools {
        public static int total = 17;
        public static long wide = 9223372036854775806L;
        public static String label = "alpha";
        public int instance = 2;
        private static int secret = 9;
        public static int plus(int value) { return total + value; }
        public int member() { return instance; }
        public static class Deep { public static int count = 33; }
    }
    static class PackageSlot { public static int value = 29; }
    private static class HiddenSlot { public static int value = 31; }
    protected static class GuardedSlot { public static int value = 37; }
    public class InnerSlot { public int read() { return 1; } }
    public static class Shadow { public static int value = 50; }
    public static Probe Shadow = new Probe(91);
    public static void changePackage(int value) { PackageSlot.value = value; }
}
