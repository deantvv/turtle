use glutin::event_loop::EventLoopProxy;
use tokio::sync::Mutex;

use crate::ipc_protocol::{
    ServerConnection,
    ServerResponse,
    TurtleProp,
    TurtlePropValue,
    PenProp,
    PenPropValue,
};
use crate::renderer_client::ClientId;

use super::super::{
    RequestRedraw,
    app::{TurtleId, TurtleDrawings},
    access_control::{AccessControl, RequiredData, RequiredTurtles},
    renderer::display_list::DisplayList,
};

pub(crate) async fn turtle_prop(
    conn: &ServerConnection,
    client_id: ClientId,
    app_control: &AccessControl,
    id: TurtleId,
    prop: TurtleProp,
) {
    let mut data = app_control.get(RequiredData {
        drawing: false,
        turtles: Some(RequiredTurtles::One(id)),
    }).await;
    let mut turtles = data.turtles_mut().await;

    let TurtleDrawings {state: turtle, current_fill_polygon, ..} = turtles.one_mut();

    use TurtleProp::*;
    use PenProp::*;
    let value = match prop {
        Pen(IsEnabled) => TurtlePropValue::Pen(PenPropValue::IsEnabled(turtle.pen.is_enabled)),
        Pen(Thickness) => TurtlePropValue::Pen(PenPropValue::Thickness(turtle.pen.thickness)),
        Pen(Color) => TurtlePropValue::Pen(PenPropValue::Color(turtle.pen.color)),
        FillColor => TurtlePropValue::FillColor(turtle.fill_color),
        IsFilling => TurtlePropValue::IsFilling(current_fill_polygon.is_some()),
        Position => TurtlePropValue::Position(turtle.position),
        PositionX => TurtlePropValue::PositionX(turtle.position.x),
        PositionY => TurtlePropValue::PositionY(turtle.position.y),
        Heading => TurtlePropValue::Heading(turtle.heading),
        Speed => TurtlePropValue::Speed(turtle.speed),
        IsVisible => TurtlePropValue::IsVisible(turtle.is_visible),
    };

    conn.send(client_id, ServerResponse::TurtleProp(id, value)).await
        .expect("unable to send response to IPC client");
}

pub(crate) async fn set_turtle_prop(
    app_control: &AccessControl,
    display_list: &Mutex<DisplayList>,
    event_loop: &Mutex<EventLoopProxy<RequestRedraw>>,
    id: TurtleId,
    prop_value: TurtlePropValue,
) {
    let mut data = app_control.get(RequiredData {
        drawing: false,
        turtles: Some(RequiredTurtles::One(id)),
    }).await;
    let mut turtles = data.turtles_mut().await;

    let TurtleDrawings {state: turtle, current_fill_polygon, ..} = turtles.one_mut();

    use TurtlePropValue::*;
    use PenPropValue::*;
    match prop_value {
        Pen(IsEnabled(is_enabled)) => turtle.pen.is_enabled = is_enabled,
        Pen(Thickness(thickness)) => turtle.pen.thickness = thickness,
        Pen(Color(color)) => turtle.pen.color = color,
        FillColor(fill_color) => {
            turtle.fill_color = fill_color;

            // Update the current fill polygon to the new color
            if let Some(poly_handle) = *current_fill_polygon {
                let mut display_list = display_list.lock().await;
                display_list.polygon_set_fill_color(poly_handle, fill_color);

                // Signal the main thread that the image has changed
                event_loop.lock().await.send_event(RequestRedraw)
                    .expect("bug: event loop closed before animation completed");
            }
        },
        IsFilling(_) => unreachable!("bug: should have used `BeginFill` and `EndFill` instead"),
        Position(_) |
        PositionX(_) |
        PositionY(_) => unreachable!("bug: should have used `MoveTo` instead"),
        Heading(_) => unreachable!("bug: should have used `RotateInPlace` instead"),
        Speed(speed) => turtle.speed = speed,
        IsVisible(is_visible) => turtle.is_visible = is_visible,
    }
}