import anonymousexposure.*;
public class ExposureCheck {
    static void check(First value, int first, int second) {
        if (value.first() != first || ((Second) value).second() != second) throw new AssertionError();
    }
    public static void main(String[] args) {
        check(ExposureKt.explicit(), 1, 2);
        check(ExposureKt.getExplicitValue(), 3, 4);
        if (ExposureKt.readPrivate() != 6) throw new AssertionError();
        check(new Holder().getExplicitValue(), 7, 8);
        check(Single.INSTANCE.getExplicitValue(), 9, 10);
        check(CompanionOwner.Companion.getExplicitValue(), 11, 12);
        System.out.println("OK");
    }
}
