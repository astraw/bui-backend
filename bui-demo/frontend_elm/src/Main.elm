port module Main exposing (..)

-- Convert to javascript with `elm-make Main.elm --output=main.js`

import Http

import Html exposing (..)
import Html.Events
import Json.Decode
import Json.Decode as Decode
import Json.Decode exposing (int, string, float, bool, nullable, Decoder, map3, map2, list, field)
import Json.Encode as Encode

import Material
import Material.Options as Options exposing (css)
import Material.Toggles as Toggles
import Material.Textfield as Textfield
import Material.Progress
import Material.Layout as Layout
import Material.Elevation as Elevation
import Material.Grid exposing (grid, offset, cell, size, Device(..))

-- type aliases for decoding messages from server
type alias ServerState = { name : String, is_recording : Bool, counter: Int }

decodeServerState : Decoder ServerState
decodeServerState =
  field "bui_backend" decodeInnerServerState

decodeInnerServerState : Decoder ServerState
decodeInnerServerState =
  map3 ServerState
    (field "name" string)
    (field "is_recording" bool)
    (field "counter" int)

type alias Model =
    { server_state : ServerState
    , fail_msg : String
    , local_name : String
    , mdl : Material.Model
    , token : String
    , event_source_connected : Bool
    }

init : ( Model, Cmd Msg )
init =
    ( { server_state = {name="", is_recording= False, counter = 0}
      , fail_msg = ""
      , local_name = ""
      , mdl = Material.model
      , token = ""
      , event_source_connected = False
      }
    , Cmd.none )

type Msg
  = NewServerState ServerState
  | NewToken String
  | ToggleRecordState Bool
  | SetNameOnServer
  | SetNameLocal String
  | Mdl (Material.Msg Msg)
  | FailedDecode String
  | EventSourceConnected Bool
  | CallbackDone (Result Http.Error String)

update : Msg -> Model -> (Model, Cmd Msg)
update msg model =
  case msg of
    NewServerState new_ss ->
      if not (model.server_state.name == new_ss.name) then
        -- set local_name from server when server_state.name changes
        ({model | server_state = new_ss, local_name = new_ss.name}, Cmd.none)
      else
        -- otherwise, keep current local_name
        ({model | server_state = new_ss}, Cmd.none)

    NewToken token ->
        ({model | token = token}, Cmd.none)

    ToggleRecordState checked -> (model, do_toggle_record_state model checked)

    SetNameOnServer -> (model, do_set_name model model.local_name)

    SetNameLocal str -> ({model | local_name = str}, Cmd.none)

    Mdl msg_ ->
        Material.update Mdl msg_ model

    FailedDecode str -> ({model | fail_msg = str}, Cmd.none)

    EventSourceConnected isConnected ->
        ({model | event_source_connected = isConnected}, Cmd.none)

    CallbackDone result ->
        (model, Cmd.none)

type alias Mdl =
    Material.Model

view : Model -> Html Msg
view model =
    Layout.render Mdl model.mdl
    [ Layout.fixedHeader
    ]
    { header = []
    , drawer = []
    , tabs = ([], [])
    , main = [
      grid []
        [ cell [ size All 2 ]
            []
        , cell [ size All 8
          , Elevation.e6
          , css "padding" "50px"
          ]
            [
              h3 [] [text "BUI - Rust Backend, Elm Frontend - Demo"]
              , div [] [
                Toggles.switch Mdl [0] model.mdl
                [ Options.onToggle (ToggleRecordState (not model.server_state.is_recording))
                , Toggles.ripple
                , Toggles.value model.server_state.is_recording
                ]
                [ text "record" ]
              ]
              , div [] [
                if model.server_state.is_recording then
                  Material.Progress.indeterminate
                else
                  Material.Progress.progress 0
              ]
              , div [] [
                Textfield.render Mdl [0] model.mdl
                  [ Textfield.label "Name"
                  , Textfield.floatingLabel
                  , Textfield.value model.local_name
                  , Options.onInput inputName
                  , Options.onBlur SetNameOnServer
                  , Options.on "keypress" (Json.Decode.andThen isEnter Html.Events.keyCode)
                  , Textfield.text_
                  ]
                  []
              ]
              , div [] [
                  text model.fail_msg
                ]
            ]
        ]
      ]}

isEnter : number -> Json.Decode.Decoder Msg
isEnter code =
   if code == 13 then
      Json.Decode.succeed SetNameOnServer
   else
      Json.Decode.fail "not Enter"

do_toggle_record_state : Model -> Bool -> Cmd Msg
do_toggle_record_state model checked =
  send_message model "set_is_recording" (Encode.bool checked)

inputName : String -> Msg
inputName name =
  SetNameLocal name

do_set_name : Model -> String -> Cmd Msg
do_set_name model value =
  send_message model "set_name" (Encode.string value)

callbackEncoded : String -> String -> Encode.Value -> Encode.Value
callbackEncoded token name args =
    let
        list =
            [ ( "token", Encode.string token )
            , ( "name", Encode.string name )
            , ( "args",  args )
            ]
    in
        list
            |> Encode.object

send_message : Model -> String -> Encode.Value -> Cmd Msg
send_message model name args =
    let
        body = Http.jsonBody (callbackEncoded model.token name args)
    in
        Http.send CallbackDone <|
            postCallback body

postCallback : Http.Body -> Http.Request String
postCallback body =
  Http.post "callback" body string

getServerStateOrFail : String -> Msg
getServerStateOrFail encoded =
  case Json.Decode.decodeString decodeServerState encoded of
    Ok (ssc) -> NewServerState ssc
    Err msg -> FailedDecode msg

port event_source_data : (String -> msg) -> Sub msg
port event_source_connected : (Bool -> msg) -> Sub msg

subscriptions : Model -> Sub Msg
subscriptions model =
  Sub.batch
      [ Layout.subs Mdl model.mdl
      , event_source_data getServerStateOrFail
      , event_source_connected EventSourceConnected
      ]

main : Program Never Model Msg
main =
    program { init = init, update = update, subscriptions = subscriptions, view = view }
