
use cursive::traits::Resizable;
use cursive::view::Margins;
use cursive::views::Button;
use cursive::views::Dialog;
use cursive::views::EditView;
use cursive::views::LinearLayout;
use cursive::views::PaddedView;
use cursive::views::ScrollView;

pub struct MultiFieldEditView {
    first: EditView,
    rest: Vec<EditView>,
}

pub fn make(values: Vec<String>) -> Dialog {
    Dialog::around(
        LinearLayout::vertical()
        .child(
            ScrollView::new({
                let mut sub = LinearLayout::vertical();

                for value in values {
                    let edit_view = EditView::new().content(value).fixed_width(32);
                    sub.add_child(PaddedView::lrtb(0, 0, 0, 1, edit_view));
                }

                sub
            })
        )
        .child(
            LinearLayout::horizontal()
            .child(Button::new("OK", |_| {}))
            .child(Button::new("Cancel", |_| {}))
            .child(Button::new("Add Field", |_| {}))
        )
    )
    .padding_lrtb(1, 1, 0, 0)
}
