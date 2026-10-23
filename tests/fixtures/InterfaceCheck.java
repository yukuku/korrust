import korrust.probes.models.RecordView;
import korrust.probes.models.TemplatesKt;

public final class InterfaceCheck {
    private static final class Typed implements RecordView.WithWeight {
        int reads;
        public int getItemCount() { reads++; return 2; }
        public String itemAt(int position) { return new String[] {"first", null}[position]; }
        public float weight(int position) { return Float.intBitsToFloat(position); }
    }
    private static final class Overridden implements RecordView {
        public int getItemCount() { return 2; }
        public String itemAt(int index) { return "x"; }
        public String label(int index) { return "override"; }
    }
    public static void main(String[] args) throws Exception {
        String[] labels = {"zero", "first", "\u0000二"};
        RecordView empty = (RecordView)InterfaceAssertions.artifacts(RecordView.class, "getItemCount", "label", "WithWeight", "weight", "LABELS", "VACANT", labels, 7, -7);
        InterfaceAssertions.record(empty == RecordView.Companion.getVACANT(), true);
        InterfaceAssertions.record(InterfaceAssertions.failure(() -> RecordView.DefaultImpls.label(null, 0)).startsWith("java.lang.NullPointerException:"), true);
        InterfaceAssertions.record(InterfaceAssertions.failure(() -> RecordView.WithWeight.DefaultImpls.label(null, 0)).startsWith("java.lang.NullPointerException:"), true);
        Overridden overridden = new Overridden();
        InterfaceAssertions.record(overridden.label(0), "override");
        InterfaceAssertions.record(RecordView.DefaultImpls.label(overridden, 0), labels[2]);
        InterfaceAssertions.record(InterfaceAssertions.call(RecordView.class.getDeclaredMethod("access$label$jd", RecordView.class, int.class), null, overridden, 0), labels[2]);
        Typed typed = new Typed();
        for (int index : new int[] {Integer.MIN_VALUE, -100, -8, -7, -4, -3, -2, -1, 0, 1, 2, 3, 65535, Integer.MAX_VALUE}) {
            InterfaceAssertions.record(InterfaceAssertions.failure(() -> empty.itemAt(index)), "java.lang.IllegalArgumentException:position=" + index + " count=7");
            for (int count : new int[] {2, 7}) {
                RecordView value = count == 2 ? typed : empty;
                String[] expected = new String[1], actual = new String[1];
                int next = index + count;
                String failure = InterfaceAssertions.failure(() -> expected[0] = next < 3 ? labels[next] : Integer.toString(next));
                int before = typed.reads;
                InterfaceAssertions.record(InterfaceAssertions.failure(() -> actual[0] = value.label(index)), failure);
                if (count == 2) InterfaceAssertions.record(typed.reads - before, 1);
                if (failure.equals("success")) {
                    InterfaceAssertions.record(actual[0], expected[0]);
                    InterfaceAssertions.record(RecordView.DefaultImpls.label(value, index), expected[0]);
                    if (count == 2) InterfaceAssertions.record(RecordView.WithWeight.DefaultImpls.label(typed, index), expected[0]);
                }
            }
            InterfaceAssertions.record(Float.floatToRawIntBits(typed.weight(index)), index);
            InterfaceAssertions.record(TemplatesKt.formatInt(index), "value=" + index);
            InterfaceAssertions.record(TemplatesKt.adjacent(index), "" + index + index);
            InterfaceAssertions.record(TemplatesKt.dollarLiteral(index), "literal=$value");
            InterfaceAssertions.record(TemplatesKt.escapedAndReal(index), "literal=$value real=" + index + " \u0000\ud800");
            InterfaceAssertions.record(TemplatesKt.unicodeDollar(index), "$value=" + index);
        }
        InterfaceAssertions.record(TemplatesKt.formatFlag(true), "flag=true");
        InterfaceAssertions.record(TemplatesKt.formatFlag(false), "flag=false");
        for (String text : new String[] {"", "\u0000", "\ud800", "\udfff", "漢😀", "$value"}) InterfaceAssertions.record(TemplatesKt.formatText(text), "text=" + text);
        InterfaceAssertions.finish();
    }
}
