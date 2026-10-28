package visibilitygeneric;

public final class GenericStorageCheck {
    public static void main(String[] args) {
        Dual<Integer, String> reader = new Dual<Integer, String>(1);
        String text = "\u0000\ud83d\ude03";
        if (reader.readAsB(new Dual<String, String>(text)) != text) throw new AssertionError();
        if (reader.readAsB(new Dual<String, String>(null)) != null) throw new AssertionError();
        try {
            reader.readAsB(null);
            throw new AssertionError();
        } catch (NullPointerException expected) {
            System.out.println("checks=3");
        }
    }
}
