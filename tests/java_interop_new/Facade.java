package newinterop;

class HiddenParent {
    public static String inheritedStatic() { return "hidden"; }
    public String getInheritedTitle() { return "visible"; }
    public static int inheritedCounter = 5;
}

public class Facade extends HiddenParent {}
