import java.lang.reflect.Field;
import kotlin.Unit;
import yuku.alkitab.base.verses.VersesController;
import yuku.alkitab.base.verses.VersesListeners;
import yuku.alkitab.base.widget.VerseInlineLinkSpan;
import yuku.alkitab.base.widget.ParallelClickData;
import yuku.alkitab.base.widget.DictionaryLinkInfo;

public final class ListenersCheck {
    private static int checks;
    private static void check(boolean value) {
        checks++;
        if (!value) throw new AssertionError("check " + checks);
    }
    public static void main(String[] args) throws Exception {
        VersesListeners empty = VersesListeners.EMPTY;
        check(empty != null && VersesListeners.EMPTY == empty);
        check(empty.getAttributeListener().getClass().getSuperclass() == VersesController.AttributeListener.class);
        check(empty.getSelectedVersesListener().getClass().getSuperclass() == VersesController.SelectedVersesListener.class);
        check(empty.getVerseScrollListener().getClass().getSuperclass() == VersesController.VerseScrollListener.class);
        check(empty.getPinDropListener().getClass().getSuperclass() == VersesController.PinDropListener.class);
        empty.getSelectedVersesListener().onNoVersesSelected();
        empty.getSelectedVersesListener().onVerseSingleClick(Integer.MIN_VALUE);
        empty.getVerseScrollListener().onVerseScroll(false, Integer.MAX_VALUE, 0.75f);
        empty.getVerseScrollListener().onScrollToTop();
        empty.getPinDropListener().onPinDropped(Integer.MIN_VALUE, Integer.MAX_VALUE);
        check(empty.getParallelListener_().invoke(new ParallelClickData() {}) == Unit.INSTANCE);
        check(empty.getDictionaryListener_().invoke(new DictionaryLinkInfo("original", "key")) == Unit.INSTANCE);
        check(empty.component1() == empty.getAttributeListener());
        check(empty.component2() == empty.getSelectedVersesListener());
        check(empty.component3() == empty.getVerseScrollListener());
        check(empty.component4() == empty.getParallelListener_());
        check(empty.component5() == empty.getInlineLinkSpanFactory_());
        check(empty.component6() == empty.getDictionaryListener_());
        check(empty.component7() == empty.getPinDropListener());
        VersesListeners copied = empty.copy(empty.component1(), empty.component2(), empty.component3(), empty.component4(), empty.component5(), empty.component6(), empty.component7());
        check(copied != empty && copied.equals(empty) && empty.equals(copied));
        check(copied.hashCode() == empty.hashCode());
        Field type = VerseInlineLinkSpan.class.getDeclaredField("type");
        Field arif = VerseInlineLinkSpan.class.getDeclaredField("arif");
        type.setAccessible(true);
        arif.setAccessible(true);
        VerseInlineLinkSpan first = null;
        for (VerseInlineLinkSpan.Type kind : VerseInlineLinkSpan.Type.values()) {
            for (int value : new int[] {Integer.MIN_VALUE, 0, Integer.MAX_VALUE}) {
                VerseInlineLinkSpan span = empty.getInlineLinkSpanFactory_().create(kind, value);
                check(span.getClass().getSuperclass() == VerseInlineLinkSpan.class);
                check(type.get(span) == kind);
                check(arif.getInt(span) == value);
                check(span != first);
                span.onClick(kind, value);
                if (first == null) first = span;
            }
        }
        check(checks == 40);
        System.out.println("listener-checks=" + checks);
    }
}
