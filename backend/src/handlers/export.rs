//! Data export handlers — CSV / Excel export for orders, trades, positions, and account data.

use std::io::Write;
use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::{HeaderMap, header::CONTENT_DISPOSITION},
    response::IntoResponse,
};
use csv::Writer;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, Order as DbOrder, PaginatorTrait, QueryFilter,
    QueryOrder,
};
use serde::Deserialize;
use uuid::Uuid;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use crate::db::order::paper_accounts;
use crate::db::order::positions;
use crate::handlers::order::{OrderResponse, PositionResponse, TradeResponse};
use crate::middleware::auth::AuthenticatedUser;
use crate::utils::error::AppError;

#[derive(Debug, Deserialize)]
pub struct ExportQuery {
    pub format: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

fn parse_format(query: &ExportQuery) -> (&str, &str) {
    match query.format.as_deref() {
        Some("xlsx") | Some("excel") => (
            "xlsx",
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        ),
        _ => ("csv", "text/csv; charset=utf-8"),
    }
}

// ─── Shared helpers ───────────────────────────────────────────────────────────

fn col_letter(col_idx: usize) -> String {
    let mut s = String::new();
    let mut n = col_idx;
    loop {
        s.push((b'A' + (n % 26) as u8) as char);
        n = n / 26;
        if n == 0 {
            break;
        }
        n -= 1;
    }
    s.chars().rev().collect()
}

fn start_file<W: Write + std::io::Seek>(
    zw: &mut ZipWriter<W>,
    name: &str,
    opts: SimpleFileOptions,
) -> Result<(), AppError> {
    zw.start_file(name, opts)
        .map_err(|e| AppError::Internal(e.to_string()))
}

// ─── XLSX static structure ────────────────────────────────────────────────────

fn write_xlsx_static_files<W: Write + std::io::Seek>(
    zw: &mut ZipWriter<W>,
    opts: SimpleFileOptions,
) -> Result<(), AppError> {
    // [Content_Types].xml
    start_file(zw, "[Content_Types].xml", opts)?;
    let ct = r#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
<Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
</Types>"#;
    zw.write_all(ct.as_bytes())
        .map_err(|e| AppError::Internal(e.to_string()))?;
    zw.flush().map_err(|e| AppError::Internal(e.to_string()))?;

    // _rels/.rels
    start_file(zw, "_rels/.rels", opts)?;
    let rels = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
</Relationships>"#;
    zw.write_all(rels.as_bytes())
        .map_err(|e| AppError::Internal(e.to_string()))?;
    zw.flush().map_err(|e| AppError::Internal(e.to_string()))?;

    // xl/workbook.xml
    start_file(zw, "xl/workbook.xml", opts)?;
    let wb = r#"<?xml version="1.0" encoding="UTF-8"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
<sheets><sheet name="Data" sheetId="1" r:id="rId1" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"/></sheets>
</workbook>"#;
    zw.write_all(wb.as_bytes())
        .map_err(|e| AppError::Internal(e.to_string()))?;
    zw.flush().map_err(|e| AppError::Internal(e.to_string()))?;

    // xl/_rels/workbook.xml.rels
    start_file(zw, "xl/_rels/workbook.xml.rels", opts)?;
    let wbrels = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
</Relationships>"#;
    zw.write_all(wbrels.as_bytes())
        .map_err(|e| AppError::Internal(e.to_string()))?;
    zw.flush().map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(())
}

// ─── Order export ─────────────────────────────────────────────────────────────

const ORDER_HEADERS: [&str; 13] = [
    "订单ID",
    "交易对",
    "方向",
    "类型",
    "价格",
    "数量",
    "已成交数量",
    "平均成交价",
    "状态",
    "模式",
    "手续费",
    "有效时间",
    "创建时间",
];

fn export_orders_csv(orders: &[OrderResponse]) -> Result<(HeaderMap, Vec<u8>), AppError> {
    let mut wtr = Writer::from_writer(vec![]);
    wtr.write_record(&ORDER_HEADERS)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    for order in orders {
        let row = [
            order.order_id.clone(),
            order.symbol.clone(),
            order.side.clone(),
            order.order_type.clone(),
            order.price.clone().unwrap_or_default(),
            order.quantity.clone(),
            order.filled_quantity.clone(),
            order.avg_fill_price.clone().unwrap_or_default(),
            order.status.clone(),
            order.mode.clone(),
            order.fee.clone(),
            order.time_in_force.clone(),
            order.created_at.clone(),
        ];
        wtr.write_record(&row)
            .map_err(|e| AppError::Internal(e.to_string()))?;
    }

    let data = wtr
        .into_inner()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok((HeaderMap::new(), data))
}

fn build_orders_sheet_xml(orders: &[OrderResponse]) -> String {
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
<sheetData>"#,
    );

    // Header row
    xml.push_str(r#"<row r="1">"#);
    for (i, h) in ORDER_HEADERS.iter().enumerate() {
        let col = col_letter(i);
        xml.push_str(&format!(
            r#"<c r="{col}1" t="inlineStr"><is><t>{h}</t></is></c>"#
        ));
    }
    xml.push_str("</row>");

    // Data rows
    for (r, order) in orders.iter().enumerate() {
        let row_num = r + 2;
        let vals = [
            order.order_id.clone(),
            order.symbol.clone(),
            order.side.clone(),
            order.order_type.clone(),
            order.price.clone().unwrap_or_default(),
            order.quantity.clone(),
            order.filled_quantity.clone(),
            order.avg_fill_price.clone().unwrap_or_default(),
            order.status.clone(),
            order.mode.clone(),
            order.fee.clone(),
            order.time_in_force.clone(),
            order.created_at.clone(),
        ];
        xml.push_str(&format!("<row r=\"{row_num}\">"));
        for (col_idx, val) in vals.iter().enumerate() {
            let col = col_letter(col_idx);
            xml.push_str(&format!(
                r#"<c r="{col}{row_num}" t="inlineStr"><is><t>{val}</t></is></c>"#
            ));
        }
        xml.push_str("</row>");
    }

    xml.push_str("</sheetData></worksheet>");
    xml
}

fn export_orders_xlsx(orders: &[OrderResponse]) -> Result<(HeaderMap, Vec<u8>), AppError> {
    let mut buf = Vec::new();
    {
        let mut zw = ZipWriter::new(std::io::Cursor::new(&mut buf));
        let opts =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        write_xlsx_static_files(&mut zw, opts).map_err(|e| AppError::Internal(e.to_string()))?;

        let sheet_xml = build_orders_sheet_xml(orders);
        start_file(&mut zw, "xl/worksheets/sheet1.xml", opts)?;
        zw.write_all(sheet_xml.as_bytes())
            .map_err(|e| AppError::Internal(e.to_string()))?;
        zw.flush().map_err(|e| AppError::Internal(e.to_string()))?;

        zw.finish().map_err(|e| AppError::Internal(e.to_string()))?;
    }

    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_DISPOSITION,
        "attachment; filename=\"orders.xlsx\"".parse().unwrap(),
    );
    headers.insert(
        "Content-Type",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
            .parse()
            .unwrap(),
    );
    Ok((headers, buf))
}

async fn fetch_orders_page(
    db: &DatabaseConnection,
    user_id: Uuid,
    page: u64,
    page_size: u64,
) -> Result<Vec<OrderResponse>, AppError> {
    use crate::db::order::Column;
    use crate::db::order::Entity as OrderEntity;

    let paginator = OrderEntity::find()
        .filter(Column::UserId.eq(user_id))
        .order_by(Column::CreatedAt, DbOrder::Desc)
        .paginate(db, page_size);

    let models = paginator
        .fetch_page(page)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(models
        .into_iter()
        .map(|m| OrderResponse {
            order_id: m.id.to_string(),
            symbol: m.symbol,
            side: serde_json::to_value(&m.side)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default(),
            order_type: serde_json::to_value(&m.order_type)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default(),
            price: m.price.map(|p| format!("{:.8}", p)),
            quantity: format!("{:.8}", m.quantity),
            filled_quantity: format!("{:.8}", m.filled_quantity),
            avg_fill_price: m.avg_fill_price.map(|p| format!("{:.8}", p)),
            status: serde_json::to_value(&m.status)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default(),
            mode: serde_json::to_value(&m.mode)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default(),
            fee: format!("{:.8}", m.fee),
            time_in_force: serde_json::to_value(&m.time_in_force)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default(),
            created_at: m.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            updated_at: m.updated_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        })
        .collect())
}

/// GET /api/v1/exports/orders
pub async fn export_orders(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Query(query): Query<ExportQuery>,
) -> Result<impl IntoResponse, AppError> {
    let page_size: u64 = query.page_size.unwrap_or(5000);
    let mut page: u64 = 1;
    let user_id = user.user_id;

    let mut all_orders: Vec<OrderResponse> = Vec::new();

    loop {
        let orders = fetch_orders_page(&db, user_id, page, page_size).await?;
        if orders.is_empty() {
            break;
        }
        let count = orders.len();
        all_orders.extend(orders);
        page += 1;
        if count < page_size as usize {
            break;
        }
    }

    let (ext, _ctype) = parse_format(&query);

    if ext == "xlsx" {
        let resp = export_orders_xlsx(&all_orders)?;
        Ok(resp)
    } else {
        let resp = export_orders_csv(&all_orders)?;
        Ok(resp)
    }
}

// ─── Trade export ─────────────────────────────────────────────────────────────

const TRADE_HEADERS: [&str; 9] = [
    "成交ID",
    "订单ID",
    "交易对",
    "方向",
    "价格",
    "数量",
    "手续费",
    "做市商",
    "成交时间",
];

fn export_trades_csv(trades: &[TradeResponse]) -> Result<(HeaderMap, Vec<u8>), AppError> {
    let mut wtr = Writer::from_writer(vec![]);
    wtr.write_record(&TRADE_HEADERS)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    for trade in trades {
        let row = [
            trade.trade_id.clone(),
            trade.order_id.clone(),
            trade.symbol.clone(),
            trade.side.clone(),
            trade.price.clone(),
            trade.quantity.clone(),
            trade.fee.clone(),
            if trade.is_maker { "是" } else { "否" }.to_string(),
            trade.created_at.clone(),
        ];
        wtr.write_record(&row)
            .map_err(|e| AppError::Internal(e.to_string()))?;
    }

    let data = wtr
        .into_inner()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok((HeaderMap::new(), data))
}

fn build_trades_sheet_xml(trades: &[TradeResponse]) -> String {
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
<sheetData>"#,
    );

    xml.push_str("<row r=\"1\">");
    for (i, h) in TRADE_HEADERS.iter().enumerate() {
        let col = col_letter(i);
        xml.push_str(&format!(
            r#"<c r="{col}1" t="inlineStr"><is><t>{h}</t></is></c>"#
        ));
    }
    xml.push_str("</row>");

    for (r, trade) in trades.iter().enumerate() {
        let row_num = r + 2;
        let vals = [
            trade.trade_id.clone(),
            trade.order_id.clone(),
            trade.symbol.clone(),
            trade.side.clone(),
            trade.price.clone(),
            trade.quantity.clone(),
            trade.fee.clone(),
            if trade.is_maker {
                "是".to_string()
            } else {
                "否".to_string()
            },
            trade.created_at.clone(),
        ];
        xml.push_str(&format!("<row r=\"{row_num}\">"));
        for (col_idx, val) in vals.iter().enumerate() {
            let col = col_letter(col_idx);
            xml.push_str(&format!(
                r#"<c r="{col}{row_num}" t="inlineStr"><is><t>{val}</t></is></c>"#
            ));
        }
        xml.push_str("</row>");
    }

    xml.push_str("</sheetData></worksheet>");
    xml
}

fn export_trades_xlsx(trades: &[TradeResponse]) -> Result<(HeaderMap, Vec<u8>), AppError> {
    let mut buf = Vec::new();
    {
        let mut zw = ZipWriter::new(std::io::Cursor::new(&mut buf));
        let opts =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        write_xlsx_static_files(&mut zw, opts).map_err(|e| AppError::Internal(e.to_string()))?;

        let sheet_xml = build_trades_sheet_xml(trades);
        start_file(&mut zw, "xl/worksheets/sheet1.xml", opts)?;
        zw.write_all(sheet_xml.as_bytes())
            .map_err(|e| AppError::Internal(e.to_string()))?;
        zw.flush().map_err(|e| AppError::Internal(e.to_string()))?;

        zw.finish().map_err(|e| AppError::Internal(e.to_string()))?;
    }

    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_DISPOSITION,
        "attachment; filename=\"trades.xlsx\"".parse().unwrap(),
    );
    headers.insert(
        "Content-Type",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
            .parse()
            .unwrap(),
    );
    Ok((headers, buf))
}

async fn fetch_trades_page(
    db: &DatabaseConnection,
    user_id: Uuid,
    page: u64,
    page_size: u64,
) -> Result<Vec<TradeResponse>, AppError> {
    use crate::db::order::trades::Column;
    use crate::db::order::trades::Entity as TradeEntity;

    let paginator = TradeEntity::find()
        .filter(Column::UserId.eq(user_id))
        .order_by(Column::CreatedAt, DbOrder::Desc)
        .paginate(db, page_size);

    let models = paginator
        .fetch_page(page)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(models
        .into_iter()
        .map(|m| TradeResponse {
            trade_id: m.id.to_string(),
            order_id: m.order_id.to_string(),
            symbol: m.symbol.clone(),
            side: serde_json::to_value(&m.side)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default(),
            price: format!("{:.8}", m.price),
            quantity: format!("{:.8}", m.quantity),
            fee: format!("{:.8}", m.fee),
            is_maker: m.is_maker,
            created_at: m.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        })
        .collect())
}

/// GET /api/v1/exports/trades
pub async fn export_trades(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Query(query): Query<ExportQuery>,
) -> Result<impl IntoResponse, AppError> {
    let page_size: u64 = query.page_size.unwrap_or(5000);
    let mut page: u64 = 1;
    let user_id = user.user_id;

    let mut all_trades: Vec<TradeResponse> = Vec::new();

    loop {
        let trades = fetch_trades_page(&db, user_id, page, page_size).await?;
        if trades.is_empty() {
            break;
        }
        let count = trades.len();
        all_trades.extend(trades);
        page += 1;
        if count < page_size as usize {
            break;
        }
    }

    let (ext, _ctype) = parse_format(&query);
    if ext == "xlsx" {
        export_trades_xlsx(&all_trades)
    } else {
        export_trades_csv(&all_trades)
    }
}

// ─── Position export ─────────────────────────────────────────────────────────

const POSITION_HEADERS: [&str; 10] = [
    "持仓ID",
    "交易对",
    "方向",
    "数量",
    "可用数量",
    "开仓均价",
    "未实现盈亏",
    "已实现盈亏",
    "模式",
    "创建时间",
];

fn build_positions_sheet_xml(positions: &[PositionResponse]) -> String {
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
<sheetData>"#,
    );

    xml.push_str("<row r=\"1\">");
    for (i, h) in POSITION_HEADERS.iter().enumerate() {
        let col = col_letter(i);
        xml.push_str(&format!(
            r#"<c r="{col}1" t="inlineStr"><is><t>{h}</t></is></c>"#
        ));
    }
    xml.push_str("</row>");

    for (r, pos) in positions.iter().enumerate() {
        let row_num = r + 2;
        let vals = [
            pos.id.clone(),
            pos.symbol.clone(),
            pos.side.clone(),
            pos.quantity.clone(),
            pos.available_quantity.clone(),
            pos.avg_entry_price.clone(),
            pos.unrealized_pnl.clone(),
            pos.realized_pnl.clone(),
            pos.mode.clone(),
            pos.created_at.clone(),
        ];
        xml.push_str(&format!("<row r=\"{row_num}\">"));
        for (col_idx, val) in vals.iter().enumerate() {
            let col = col_letter(col_idx);
            xml.push_str(&format!(
                r#"<c r="{col}{row_num}" t="inlineStr"><is><t>{val}</t></is></c>"#
            ));
        }
        xml.push_str("</row>");
    }

    xml.push_str("</sheetData></worksheet>");
    xml
}

fn export_positions_xlsx(positions: &[PositionResponse]) -> Result<(HeaderMap, Vec<u8>), AppError> {
    let mut buf = Vec::new();
    {
        let mut zw = ZipWriter::new(std::io::Cursor::new(&mut buf));
        let opts =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        write_xlsx_static_files(&mut zw, opts).map_err(|e| AppError::Internal(e.to_string()))?;

        let sheet_xml = build_positions_sheet_xml(positions);
        start_file(&mut zw, "xl/worksheets/sheet1.xml", opts)?;
        zw.write_all(sheet_xml.as_bytes())
            .map_err(|e| AppError::Internal(e.to_string()))?;
        zw.flush().map_err(|e| AppError::Internal(e.to_string()))?;

        zw.finish().map_err(|e| AppError::Internal(e.to_string()))?;
    }

    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_DISPOSITION,
        "attachment; filename=\"positions.xlsx\"".parse().unwrap(),
    );
    headers.insert(
        "Content-Type",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
            .parse()
            .unwrap(),
    );
    Ok((headers, buf))
}

fn export_positions_csv(positions: &[PositionResponse]) -> Result<(HeaderMap, Vec<u8>), AppError> {
    let mut wtr = Writer::from_writer(vec![]);
    wtr.write_record(&POSITION_HEADERS)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    for pos in positions {
        let row = [
            pos.id.clone(),
            pos.symbol.clone(),
            pos.side.clone(),
            pos.quantity.clone(),
            pos.available_quantity.clone(),
            pos.avg_entry_price.clone(),
            pos.unrealized_pnl.clone(),
            pos.realized_pnl.clone(),
            pos.mode.clone(),
            pos.created_at.clone(),
        ];
        wtr.write_record(&row)
            .map_err(|e| AppError::Internal(e.to_string()))?;
    }

    let data = wtr
        .into_inner()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok((HeaderMap::new(), data))
}

async fn fetch_positions_page(
    db: &DatabaseConnection,
    user_id: Uuid,
    page: u64,
    page_size: u64,
) -> Result<Vec<PositionResponse>, AppError> {
    use crate::db::order::positions::Column;
    use crate::db::order::positions::Entity as PositionEntity;

    let paginator = PositionEntity::find()
        .filter(Column::UserId.eq(user_id))
        .order_by(Column::UpdatedAt, DbOrder::Desc)
        .paginate(db, page_size);

    let models = paginator
        .fetch_page(page)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(models
        .into_iter()
        .map(|m| PositionResponse {
            id: m.id.to_string(),
            symbol: m.symbol.clone(),
            side: serde_json::to_value(&m.side)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default(),
            quantity: format!("{:.8}", m.quantity),
            available_quantity: format!("{:.8}", m.available_quantity),
            avg_entry_price: format!("{:.8}", m.avg_entry_price),
            unrealized_pnl: format!("{:.8}", m.unrealized_pnl),
            realized_pnl: format!("{:.8}", m.realized_pnl),
            mode: serde_json::to_value(&m.mode)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default(),
            created_at: m.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            updated_at: m.updated_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        })
        .collect())
}

/// GET /api/v1/exports/positions
pub async fn export_positions(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Query(query): Query<ExportQuery>,
) -> Result<impl IntoResponse, AppError> {
    let page_size: u64 = query.page_size.unwrap_or(5000);
    let mut page: u64 = 1;
    let user_id = user.user_id;

    let mut all_positions: Vec<PositionResponse> = Vec::new();

    loop {
        let positions = fetch_positions_page(&db, user_id, page, page_size).await?;
        if positions.is_empty() {
            break;
        }
        let count = positions.len();
        all_positions.extend(positions);
        page += 1;
        if count < page_size as usize {
            break;
        }
    }

    let (ext, _ctype) = parse_format(&query);
    if ext == "xlsx" {
        export_positions_xlsx(&all_positions)
    } else {
        export_positions_csv(&all_positions)
    }
}

// ─── Account export ───────────────────────────────────────────────────────────

fn build_account_sheet_xml(
    account: &Option<paper_accounts::Model>,
    positions: &[positions::Model],
) -> String {
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
<sheetData>"#,
    );
    let mut row: usize = 1;

    // Account info section
    if let Some(acc) = account {
        let fields = [
            ("用户ID", acc.user_id.to_string()),
            ("总权益", acc.balance.to_string()),
            ("冻结资金", acc.frozen_balance.to_string()),
            ("初始资金", acc.initial_balance.to_string()),
            (
                "更新时间",
                acc.updated_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            ),
        ];
        for (k, v) in fields {
            xml.push_str(&format!(
                "<row r=\"{row}\"><c r=\"A{row}\" t=\"inlineStr\"><is><t>{k}</t></is></c><c r=\"B{row}\" t=\"inlineStr\"><is><t>{v}</t></is></c></row>"
            ));
            row += 1;
        }
    } else {
        xml.push_str(&format!(
            "<row r=\"{row}\"><c r=\"A{row}\" t=\"inlineStr\"><is><t>账户未找到</t></is></c></row>"
        ));
        row += 1;
    }

    // Positions sub-table
    xml.push_str(&format!(
        "<row r=\"{row}\"><c r=\"A{row}\" t=\"inlineStr\"><is><t>=== 持仓 ===</t></is></c></row>"
    ));
    row += 1;

    xml.push_str(&format!("<row r=\"{row}\">"));
    for (col_idx, h) in POSITION_HEADERS.iter().enumerate() {
        let col = col_letter(col_idx);
        xml.push_str(&format!(
            "<c r=\"{col}{row}\" t=\"inlineStr\"><is><t>{h}</t></is></c>"
        ));
    }
    xml.push_str("</row>");
    row += 1;

    for pos in positions {
        let vals = [
            pos.id.to_string(),
            pos.symbol.clone(),
            format!("{:?}", pos.side),
            pos.quantity.to_string(),
            pos.available_quantity.to_string(),
            pos.avg_entry_price.to_string(),
            pos.unrealized_pnl.to_string(),
            pos.realized_pnl.to_string(),
            format!("{:?}", pos.mode),
            pos.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        ];
        xml.push_str(&format!("<row r=\"{row}\">"));
        for (col_idx, val) in vals.iter().enumerate() {
            let col = col_letter(col_idx);
            xml.push_str(&format!(
                "<c r=\"{col}{row}\" t=\"inlineStr\"><is><t>{val}</t></is></c>"
            ));
        }
        xml.push_str("</row>");
        row += 1;
    }

    xml.push_str("</sheetData></worksheet>");
    xml
}

fn export_account_xlsx(
    account: &Option<paper_accounts::Model>,
    positions: &[positions::Model],
) -> Result<(HeaderMap, Vec<u8>), AppError> {
    let mut buf = Vec::new();
    {
        let mut zw = ZipWriter::new(std::io::Cursor::new(&mut buf));
        let opts =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        write_xlsx_static_files(&mut zw, opts).map_err(|e| AppError::Internal(e.to_string()))?;

        let sheet_xml = build_account_sheet_xml(account, positions);
        start_file(&mut zw, "xl/worksheets/sheet1.xml", opts)?;
        zw.write_all(sheet_xml.as_bytes())
            .map_err(|e| AppError::Internal(e.to_string()))?;
        zw.flush().map_err(|e| AppError::Internal(e.to_string()))?;

        zw.finish().map_err(|e| AppError::Internal(e.to_string()))?;
    }

    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_DISPOSITION,
        "attachment; filename=\"account.xlsx\"".parse().unwrap(),
    );
    headers.insert(
        "Content-Type",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
            .parse()
            .unwrap(),
    );
    Ok((headers, buf))
}

fn export_account_csv(
    account: &Option<paper_accounts::Model>,
    positions: &[positions::Model],
) -> Result<(HeaderMap, Vec<u8>), AppError> {
    let mut wtr = Writer::from_writer(vec![]);

    wtr.write_record(&["=== 账户信息 ==="])
        .map_err(|e| AppError::Internal(e.to_string()))?;
    wtr.write_record(&["项目", "值"])
        .map_err(|e| AppError::Internal(e.to_string()))?;

    if let Some(acc) = account {
        let fields = [
            ("用户ID", acc.user_id.to_string()),
            ("总权益", acc.balance.to_string()),
            ("冻结资金", acc.frozen_balance.to_string()),
            ("初始资金", acc.initial_balance.to_string()),
            (
                "更新时间",
                acc.updated_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            ),
        ];
        for (k, v) in fields {
            wtr.write_record(&[k, v.as_str()])
                .map_err(|e| AppError::Internal(e.to_string()))?;
        }
    } else {
        wtr.write_record(&["账户", "未找到"])
            .map_err(|e| AppError::Internal(e.to_string()))?;
    }

    wtr.write_record::<&[&str], _>(&[])
        .map_err(|e| AppError::Internal(e.to_string()))?;
    wtr.write_record(&["=== 持仓信息 ==="])
        .map_err(|e| AppError::Internal(e.to_string()))?;
    wtr.write_record(&POSITION_HEADERS)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    for pos in positions {
        wtr.write_record(&[
            &pos.id.to_string(),
            &pos.symbol,
            &format!("{:?}", pos.side),
            &pos.quantity.to_string(),
            &pos.available_quantity.to_string(),
            &pos.avg_entry_price.to_string(),
            &pos.unrealized_pnl.to_string(),
            &pos.realized_pnl.to_string(),
            &format!("{:?}", pos.mode),
            &pos.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        ])
        .map_err(|e| AppError::Internal(e.to_string()))?;
    }

    let data = wtr
        .into_inner()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_DISPOSITION,
        "attachment; filename=\"account.csv\"".parse().unwrap(),
    );
    headers.insert("Content-Type", "text/csv; charset=utf-8".parse().unwrap());
    Ok((headers, data))
}

async fn fetch_account(
    db: &DatabaseConnection,
    user_id: Uuid,
) -> Result<Option<paper_accounts::Model>, AppError> {
    use crate::db::order::paper_accounts::Column;
    use crate::db::order::paper_accounts::Entity as AccountEntity;

    let account = AccountEntity::find()
        .filter(Column::UserId.eq(user_id))
        .one(db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(account)
}

async fn fetch_all_positions(
    db: &DatabaseConnection,
    user_id: Uuid,
) -> Result<Vec<positions::Model>, AppError> {
    use crate::db::order::positions::Column;
    use crate::db::order::positions::Entity as PositionEntity;

    let positions = PositionEntity::find()
        .filter(Column::UserId.eq(user_id))
        .order_by(Column::UpdatedAt, DbOrder::Desc)
        .all(db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(positions)
}

/// GET /api/v1/exports/account
pub async fn export_account(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Query(query): Query<ExportQuery>,
) -> Result<impl IntoResponse, AppError> {
    let user_id = user.user_id;

    let account = fetch_account(&*db, user_id).await?;
    let positions = fetch_all_positions(&*db, user_id).await?;

    let (ext, _ctype) = parse_format(&query);
    if ext == "xlsx" {
        export_account_xlsx(&account, &positions)
    } else {
        export_account_csv(&account, &positions)
    }
}
