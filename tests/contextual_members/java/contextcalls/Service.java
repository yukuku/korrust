package contextcalls;
public class Service {
    public static String log = "";
    public Leaf fixedLeaf() { return new Leaf(1); }
    public static Service receiver() { log += "R"; return new Service(); }
    public static int argument() { log += "A"; return 3; }
    @SuppressWarnings("unchecked") public <T extends Item> T lookup(int kind) {
        log += "L";
        return (T) (kind == 0 ? null : kind == 2 ? new Other(2) : new Leaf(kind));
    }
    public <T extends Item> T mixed(int kind) { return lookup(kind); }
    @SuppressWarnings("unchecked") public <T extends Number> T mixed(String value) { return (T) Integer.valueOf(value.length()); }
    public <T extends Item> T lookup() { return lookup(1); }
    public <T extends Item> T lookup(String value) { return lookup(value.length()); }
}
