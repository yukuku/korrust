package star.library;
class HiddenBase {
    public static int inherited(int value) { return value + 31; }
    public static int inheritedState = 37;
}
public class Derived extends HiddenBase {}
