package yuku.alkitab.base.appwidget;
public class ActualCalls {
    public static void main(String[] args) {
        if (ActualConfigurationKt.actualConfiguration() != 25) throw new AssertionError("real internal data class");
        System.out.println("actual-internal-configuration-ok");
    }
}
