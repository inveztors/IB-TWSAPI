//! Functions for processing messages
use std::any::Any;
use std::collections::HashSet;
use std::convert::TryInto;
use std::io::Write;
use std::string::String;
use std::vec::Vec;

use ascii;
use ascii::AsAsciiStr;
use rust_decimal::Decimal;

use log::*;
use num_derive::FromPrimitive;

use crate::core::common::{
    BarData, CommissionReport, DepthMktDataDescription, FaDataType, FamilyCode, HistogramData,
    HistoricalTick, HistoricalTickBidAsk, HistoricalTickLast, NewsProvider, PriceIncrement,
    RealTimeBar, SmartComponent, TickAttrib, TickAttribBidAsk, TickAttribLast, TickByTickType,
    TickMsgType, TickType, UNSET_DOUBLE, UNSET_INTEGER,
};
use crate::core::contract::{
    Contract, ContractDescription, ContractDetails, ContractPreamble, DeltaNeutralContract,
};
use crate::core::errors::IBKRApiLibError;
use crate::core::execution::{Execution, ExecutionFilter};
use crate::core::order::{Order, OrderState, SoftDollarTier};
use crate::core::scanner::ScannerSubscription;
use serde::Deserialize;
use serde::Serialize;
use strum_macros::Display;

//==================================================================================================
trait EClientMsgSink {
    fn server_version(version: i32, time: &str);
    fn redirect(host: &str);
}

//==================================================================================================
/// FA msg data types
pub enum FAMessageDataTypes {
    Groups = 1,
    Profiles = 2,
    Aliases = 3,
}

#[derive(FromPrimitive)]
#[repr(i32)]
pub enum ServerRspMsgDiscriminants {
    TickPrice = 1,
    TickSize = 2,
    OrderStatus = 3,
    ErrMsg = 4,
    OpenOrder = 5,
    AcctValue = 6,
    PortfolioValue = 7,
    AcctUpdateTime = 8,
    NextValidId = 9,
    ContractData = 10,
    ExecutionData = 11,
    MarketDepth = 12,
    MarketDepthL2 = 13,
    NewsBulletins = 14,
    ManagedAccts = 15,
    ReceiveFa = 16,
    HistoricalData = 17,
    BondContractData = 18,
    ScannerParameters = 19,
    ScannerData = 20,
    TickOptionComputation = 21,
    TickGeneric = 45,
    TickString = 46,
    TickEfp = 47,
    CurrentTime = 49,
    RealTimeBars = 50,
    FundamentalData = 51,
    ContractDataEnd = 52,
    OpenOrderEnd = 53,
    AcctDownloadEnd = 54,
    ExecutionDataEnd = 55,
    DeltaNeutralValidation = 56,
    TickSnapshotEnd = 57,
    MarketDataType = 58,
    CommissionReport = 59,
    PositionData = 61,
    PositionEnd = 62,
    AccountSummary = 63,
    AccountSummaryEnd = 64,
    VerifyMessageApi = 65,
    VerifyCompleted = 66,
    DisplayGroupList = 67,
    DisplayGroupUpdated = 68,
    VerifyAndAuthMessageApi = 69,
    VerifyAndAuthCompleted = 70,
    PositionMulti = 71,
    PositionMultiEnd = 72,
    AccountUpdateMulti = 73,
    AccountUpdateMultiEnd = 74,
    SecurityDefinitionOptionParameter = 75,
    SecurityDefinitionOptionParameterEnd = 76,
    SoftDollarTiers = 77,
    FamilyCodes = 78,
    SymbolSamples = 79,
    MktDepthExchanges = 80,
    TickReqParams = 81,
    SmartComponents = 82,
    NewsArticle = 83,
    TickNews = 84,
    NewsProviders = 85,
    HistoricalNews = 86,
    HistoricalNewsEnd = 87,
    HeadTimestamp = 88,
    HistogramData = 89,
    HistoricalDataUpdate = 90,
    RerouteMktDataReq = 91,
    RerouteMktDepthReq = 92,
    MarketRule = 93,
    Pnl = 94,
    PnlSingle = 95,
    HistoricalTicks = 96,
    HistoricalTicksBidAsk = 97,
    HistoricalTicksLast = 98,
    TickByTick = 99,
    OrderBound = 100,
    CompletedOrder = 101,
    CompletedOrdersEnd = 102,
}

#[derive(Clone, Serialize, Deserialize, Debug, Display)]
pub enum ServerRspMsg {
    TickPrice {
        req_id: i32,
        tick_type: TickType,
        price: f64,
        tick_attr: TickAttrib,
    },
    TickSize {
        req_id: i32,
        tick_type: TickType,
        size: i32,
    },
    OrderStatus {
        order_id: i32,
        status: String,
        filled: f64,
        remaining: f64,
        avg_fill_price: f64,
        perm_id: i32,
        parent_id: i32,
        last_fill_price: f64,
        client_id: i32,
        why_held: String,
        mkt_cap_price: f64,
    },
    ErrMsg {
        req_id: i32,
        error_code: i32,
        error_str: String,
    },
    OpenOrder {
        order_id: i32,
        contract: Contract,
        order: Order,
        order_state: OrderState,
    },
    AcctValue {
        key: String,
        val: String,
        currency: String,
        account_name: String,
    },
    PortfolioValue {
        contract: Contract,
        position: f64,
        market_price: f64,
        market_value: f64,
        average_cost: f64,
        unrealized_pnl: f64,
        realized_pnl: f64,
        account_name: String,
    },
    AcctUpdateTime {
        time_stamp: String,
    },
    NextValidId {
        order_id: i32,
    },
    ContractData {
        req_id: i32,
        contract_details: ContractDetails,
    },
    ExecutionData {
        req_id: i32,
        contract: Contract,
        execution: Execution,
    },
    MarketDepth {
        req_id: i32,
        position: i32,
        operation: i32,
        side: i32,
        price: f64,
        size: i32,
    },
    MarketDepthL2 {
        req_id: i32,
        position: i32,
        market_maker: String,
        operation: i32,
        side: i32,
        price: f64,
        size: i32,
        is_smart_depth: bool,
    },
    NewsBulletins {
        msg_id: i32,
        msg_type: i32,
        news_message: String,
        origin_exch: String,
    },
    ManagedAccts {
        accounts_list: String,
    },
    ReceiveFa {
        fa_data: FaDataType,
        cxml: String,
    },
    HistoricalData {
        req_id: i32,
        bar: BarData,
    },
    BondContractData {
        req_id: i32,
        contract_details: ContractDetails,
    },

    ScannerParameters {
        xml: String,
    },
    ScannerData {
        req_id: i32,
        rank: i32,
        contract_details: ContractDetails,
        distance: String,
        benchmark: String,
        projection: String,
        legs_str: String,
    },
    TickOptionComputation {
        ticker_id: i32,
        tick_type: TickType,
        implied_vol: f64,
        delta: f64,
        opt_price: f64,
        pv_dividend: f64,
        gamma: f64,
        vega: f64,
        theta: f64,
        und_price: f64,
    },
    TickGeneric {
        ticker_id: i32,
        tick_type: TickType,
        value: f64,
    },
    TickString {
        req_id: i32,
        tick_type: TickType,
        value: String,
    },
    TickEfp {
        ticker_id: i32,
        tick_type: TickType,
        basis_points: f64,
        formatted_basis_points: String,
        implied_futures_price: f64,
        hold_days: i32,
        future_last_trade_date: String,
        dividend_impact: f64,
        dividends_to_last_trade_date: f64,
    },
    CurrentTime {
        time: i64,
    },
    RealTimeBars {
        req_id: i32,
        bar: RealTimeBar,
    },
    FundamentalData {
        req_id: i32,
        data: String,
    },
    ContractDataEnd {
        req_id: i32,
    },
    OpenOrderEnd,
    AcctDownloadEnd {
        account_name: String,
    },
    ExecutionDataEnd {
        req_id: i32,
    },
    DeltaNeutralValidation {
        req_id: i32,
        delta_neutral_contract: DeltaNeutralContract,
    },
    ScannerDataEnd {
        req_id: i32,
    },
    TickSnapshotEnd {
        req_id: i32,
    },
    MarketDataType {
        req_id: i32,
        market_data_type: i32,
    },
    CommissionReport {
        commission_report: CommissionReport,
    },
    PositionData {
        account: String,
        contract: Contract,
        position: f64,
        avg_cost: f64,
    },
    PositionEnd,
    AccountSummary {
        req_id: i32,
        account: String,
        tag: String,
        value: String,
        currency: String,
    },
    AccountSummaryEnd {
        req_id: i32,
    },
    VerifyMessageApi {
        api_data: String,
    },
    VerifyCompleted {
        is_successful: bool,
        error_text: String,
    },
    DisplayGroupList {
        req_id: i32,
        groups: String,
    },
    DisplayGroupUpdated {
        req_id: i32,
        contract_info: String,
    },
    VerifyAndAuthMessageApi {
        api_data: String,
        xyz_challenge: String,
    },
    VerifyAndAuthCompleted {
        is_successful: bool,
        error_text: String,
    },
    PositionMulti {
        req_id: i32,
        account: String,
        model_code: String,
        contract: Contract,
        position: f64,
        avg_cost: f64,
    },
    PositionMultiEnd {
        req_id: i32,
    },
    AccountUpdateMulti {
        req_id: i32,
        account: String,
        model_code: String,
        key: String,
        value: String,
        currency: String,
    },
    AccountUpdateMultiEnd {
        req_id: i32,
    },
    SecurityDefinitionOptionParameter {
        req_id: i32,
        exchange: String,
        underlying_con_id: i32,
        trading_class: String,
        multiplier: String,
        expirations: HashSet<String>,
        strikes: HashSet<Decimal>,
    },
    SecurityDefinitionOptionParameterEnd {
        req_id: i32,
    },
    SoftDollarTiers {
        req_id: i32,
        tiers: Vec<SoftDollarTier>,
    },
    FamilyCodes {
        family_codes: Vec<FamilyCode>,
    },
    SymbolSamples {
        req_id: i32,
        contract_descriptions: Vec<ContractDescription>,
    },
    MktDepthExchanges {
        depth_mkt_data_descriptions: Vec<DepthMktDataDescription>,
    },
    TickReqParams {
        ticker_id: i32,
        min_tick: f64,
        bbo_exchange: String,
        snapshot_permissions: i32,
    },
    SmartComponents {
        req_id: i32,
        smart_components: Vec<SmartComponent>,
    },
    NewsArticle {
        req_id: i32,
        article_type: i32,
        article_text: String,
    },
    TickNews {
        ticker_id: i32,
        time_stamp: i32,
        provider_code: String,
        article_id: String,
        headline: String,
        extra_data: String,
    },
    NewsProviders {
        news_providers: Vec<NewsProvider>,
    },
    HistoricalNews {
        req_id: i32,
        time: String,
        provider_code: String,
        article_id: String,
        headline: String,
    },
    HistoricalNewsEnd {
        req_id: i32,
        has_more: bool,
    },
    HeadTimestamp {
        req_id: i32,
        head_timestamp: String,
    },
    HistogramData {
        req_id: i32,
        items: Vec<HistogramData>,
    },
    HistoricalDataUpdate {
        req_id: i32,
        bar: BarData,
    },
    RerouteMktDataReq {
        req_id: i32,
        con_id: i32,
        exchange: String,
    },
    RerouteMktDepthReq {
        req_id: i32,
        con_id: i32,
        exchange: String,
    },
    MarketRule {
        market_rule_id: i32,
        price_increments: Vec<PriceIncrement>,
    },
    Pnl {
        req_id: i32,
        daily_pnl: f64,
        unrealized_pnl: f64,
        realized_pnl: f64,
    },
    PnlSingle {
        req_id: i32,
        pos: i32,
        daily_pnl: f64,
        unrealized_pnl: f64,
        realized_pnl: f64,
        value: f64,
    },
    HistoricalTicks {
        req_id: i32,
        ticks: Vec<HistoricalTick>,
        done: bool,
    },
    HistoricalTicksBidAsk {
        req_id: i32,
        ticks: Vec<HistoricalTickBidAsk>,
        done: bool,
    },
    HistoricalTicksLast {
        req_id: i32,
        ticks: Vec<HistoricalTickLast>,
        done: bool,
    },
    TickByTick {
        req_id: i32,
        tick_type: i32,
        time: i64,
        tick_msg: TickMsgType,
    },
    OrderBound {
        req_id: i32,
        api_client_id: i32,
        api_order_id: i32,
    },
    CompletedOrder {
        contract: Contract,
        order: Order,
        order_state: OrderState,
    },
    CompletedOrdersEnd,
    HistoricalDataEnd {
        req_id: i32,
        start: String,
        end: String,
    },
}

#[derive(FromPrimitive)]
#[repr(i32)]
pub enum ServerReqMsgDiscriminants {
    ReqMktData = 1,
    CancelMktData = 2,
    PlaceOrder = 3,
    CancelOrder = 4,
    ReqOpenOrders = 5,
    ReqAcctData = 6,
    ReqExecutions = 7,
    ReqIds = 8,
    ReqContractData = 9,
    ReqMktDepth = 10,
    CancelMktDepth = 11,
    ReqNewsBulletins = 12,
    CancelNewsBulletins = 13,
    SetServerLoglevel = 14,
    ReqAutoOpenOrders = 15,
    ReqAllOpenOrders = 16,
    ReqManagedAccts = 17,
    ReqFa = 18,
    ReplaceFa = 19,
    ReqHistoricalData = 20,
    ExerciseOptions = 21,
    ReqScannerSubscription = 22,
    CancelScannerSubscription = 23,
    ReqScannerParameters = 24,
    CancelHistoricalData = 25,
    ReqCurrentTime = 49,
    ReqRealTimeBars = 50,
    CancelRealTimeBars = 51,
    ReqFundamentalData = 52,
    CancelFundamentalData = 53,
    ReqCalcImpliedVolat = 54,
    ReqCalcOptionPrice = 55,
    CancelCalcImpliedVolat = 56,
    CancelCalcOptionPrice = 57,
    ReqGlobalCancel = 58,
    ReqMarketDataType = 59,
    ReqPositions = 61,
    ReqAccountSummary = 62,
    CancelAccountSummary = 63,
    CancelPositions = 64,
    VerifyRequest = 65,
    VerifyMessage = 66,
    QueryDisplayGroups = 67,
    SubscribeToGroupEvents = 68,
    UpdateDisplayGroup = 69,
    UnsubscribeFromGroupEvents = 70,
    StartApi = 71,
    VerifyAndAuthRequest = 72,
    VerifyAndAuthMessage = 73,
    ReqPositionsMulti = 74,
    CancelPositionsMulti = 75,
    ReqAccountUpdatesMulti = 76,
    CancelAccountUpdatesMulti = 77,
    ReqSecDefOptParams = 78,
    ReqSoftDollarTiers = 79,
    ReqFamilyCodes = 80,
    ReqMatchingSymbols = 81,
    ReqMktDepthExchanges = 82,
    ReqSmartComponents = 83,
    ReqNewsArticle = 84,
    ReqNewsProviders = 85,
    ReqHistoricalNews = 86,
    ReqHeadTimestamp = 87,
    ReqHistogramData = 88,
    CancelHistogramData = 89,
    CancelHeadTimestamp = 90,
    ReqMarketRule = 91,
    ReqPnl = 92,
    CancelPnl = 93,
    ReqPnlSingle = 94,
    CancelPnlSingle = 95,
    ReqHistoricalTicks = 96,
    ReqTickByTickData = 97,
    CancelTickByTickData = 98,
    ReqCompletedOrders = 99,
}

#[derive(Clone, Serialize, Deserialize, Debug, Display)]
pub enum ServerReqMsg {
    ReqMktData {
        version: i32,
        req_id: i32,
        contract: ContractPreamble,
        trading_class: String,
        generic_tick_list: String,
        snapshot: bool,
        regulatory_snapshot: bool,
        mkt_data_options: String, // internal use only, serialize as empty string field
    },
    CancelMktData {
        version: i32,
        req_id: i32,
    },
    PlaceOrder {
        version: i32,
        order_id: i32,
        contract: ContractPreamble,
        trading_class: String,
        sec_id_type: String,
        sec_id: String,
        order: Order,
    },
    CancelOrder {
        version: i32,
        order_id: i32,
    },
    ReqOpenOrders {
        version: i32,
    },
    ReqAcctData {
        version: i32,
        subscribe: bool,
        acct_code: String,
    },
    ReqExecutions {
        version: i32,
        req_id: i32,
        exec_filter: ExecutionFilter,
    },
    ReqIds {
        version: i32,
        num_ids: i32,
    },
    ReqContractData {
        version: i32,
        req_id: i32,
        contract: ContractPreamble,
        trading_class: String,
        include_expired: bool,
        sec_id_type: String,
        sec_id: String,
    },
    ReqMktDepth {
        version: i32,
        req_id: i32,
        contract: ContractPreamble,
        trading_class: String,
        num_rows: i32,
        is_smart_depth: bool,
        mkt_depth_options: String,
    },
    CancelMktDepth {
        version: i32,
        req_id: i32,
        is_smart_depth: bool,
    },
    ReqNewsBulletins {
        version: i32,
        all_msgs: bool,
    },
    CancelNewsBulletins {
        version: i32,
    },
    SetServerLoglevel {
        version: i32,
        log_level: i32,
    },
    ReqAutoOpenOrders {
        version: i32,
        auto_bind: bool,
    },
    ReqAllOpenOrders {
        version: i32,
    },
    ReqManagedAccts {
        version: i32,
    },
    ReqFa {
        version: i32,
        fa_data: i32,
    },
    ReplaceFa {
        version: i32,
        fa_data: i32,
        cxml: String,
    },
    ReqHistoricalData {
        version: i32,
        req_id: i32,
        contract: ContractPreamble,
        trading_class: String,
        include_expired: bool,
        keep_up_to_date: bool,
        chart_options: String,
    },
    ExerciseOptions {
        version: i32,
        req_id: i32,
        contract: ContractPreamble,
        trading_class: String,
        exercise_action: i32,
        exercise_quantity: i32,
        account: String,
        over_ride: i32,
    },
    ReqScannerSubscription {
        version: i32,
        req_id: i32,
        subscription: ScannerSubscription,
        scanner_subscription_filter: String,
        scanner_subscription_options: String,
    },
    CancelScannerSubscription {
        version: i32,
        req_id: i32,
    },
    ReqScannerParameters {
        version: i32,
    },
    CancelHistoricalData {
        version: i32,
        req_id: i32,
    },
    ReqCurrentTime {
        version: i32,
    },
    ReqRealTimeBars {
        version: i32,
        req_id: i32,
        contract: ContractPreamble,
        trading_class: String,
        bar_size: i32,
        what_to_show: String,
        use_rth: bool,
        real_time_bars_options: String,
    },
    CancelRealTimeBars {
        version: i32,
        req_id: i32,
    },
    ReqFundamentalData {
        version: i32,
        req_id: i32,
        contract: ContractPreamble,
        report_type: String,
        tags_value_count: i32,
        fund_data_opt: String,
    },
    CancelFundamentalData {
        version: i32,
        req_id: i32,
    },
    ReqCalcImpliedVolat {
        version: i32,
        req_id: i32,
        contract: ContractPreamble, // abreviated contract field
        trading_class: String,
        option_price: f64,
        under_price: f64,
        tag_values_cnt: usize,
        impl_vol_opt: String,
    },
    ReqCalcOptionPrice {
        version: i32,
        req_id: i32,
        contract: ContractPreamble, // abreviated contract field
        trading_class: String,
        volatility: f64,
        under_price: f64,
        tag_values_cnt: usize,
        opt_prc_opt: String,
    },
    CancelCalcImpliedVolat {
        version: i32,
        req_id: i32,
    },
    CancelCalcOptionPrice {
        version: i32,
        req_id: i32,
    },
    ReqGlobalCancel {
        version: i32,
    },
    ReqMarketDataType {
        version: i32,
        market_data_type: i32,
    },
    ReqPositions {
        version: i32,
    },
    ReqAccountSummary {
        version: i32,
        req_id: i32,
        group_name: String,
        tags: String,
    },
    CancelAccountSummary {
        version: i32,
        req_id: i32,
    },
    CancelPositions {
        version: i32,
    },
    VerifyRequest {
        version: i32,
        api_name: String,
        api_version: String,
    },
    VerifyMessage {
        version: i32,
        api_data: String,
    },
    QueryDisplayGroups {
        version: i32,
        req_id: i32,
    },
    SubscribeToGroupEvents {
        version: i32,
        req_id: i32,
        group_id: i32,
    },
    UpdateDisplayGroup {
        version: i32,
        req_id: i32,
        contract_info: String,
    },
    UnsubscribeFromGroupEvents {
        version: i32,
        req_id: i32,
    },
    StartApi {
        version: i32,
        client_id: String,
    },
    VerifyAndAuthRequest {
        version: i32,
        api_name: String,
        api_version: String,
        opaque_isv_key: String,
    },
    VerifyAndAuthMessage {
        version: i32,
        api_data: String,
        xyz_response: String,
    },
    ReqPositionsMulti {
        version: i32,
        req_id: i32,
        account: String,
        model_code: String,
    },
    CancelPositionsMulti {
        version: i32,
        req_id: i32,
    },
    ReqAccountUpdatesMulti {
        version: i32,
        req_id: i32,
        account: String,
        model_code: String,
        ledger_and_nlv: bool,
    },
    CancelAccountUpdatesMulti {
        version: i32,
        req_id: i32,
    },
    ReqSecDefOptParams {
        req_id: i32,
        underlying_symbol: String,
        fut_fop_exchange: String,
        underlying_sec_type: String,
        underlying_con_id: i32,
    },
    ReqSoftDollarTiers {
        req_id: i32,
    },
    ReqFamilyCodes,
    ReqMatchingSymbols {
        req_id: i32,
        pattern: String,
    },
    ReqMktDepthExchanges,
    ReqSmartComponents {
        req_id: i32,
        bbo_exchange: String,
    },
    ReqNewsArticle {
        req_id: i32,
        provider_code: String,
        article_id: String,
        news_article_options: String,
    },
    ReqNewsProviders,
    ReqHistoricalNews {
        req_id: i32,
        con_id: i32,
        provider_codes: String,
        start_date_time: String,
        end_date_time: String,
        total_results: i32,
        historical_news_options: String,
    },
    ReqHeadTimestamp {
        req_id: i32,
        contract: ContractPreamble,
        trading_class: String,
        include_expired: bool,
        use_rth: i32,
        what_to_show: String,
        format_date: i32,
    },
    ReqHistogramData {
        ticker_id: i32,
        contract: ContractPreamble,
        trading_class: String,
        include_expired: bool,
        use_rth: bool,
        time_period: String,
    },
    CancelHistogramData {
        ticker_id: i32,
    },
    CancelHeadTimestamp {
        req_id: i32,
    },
    ReqMarketRule {
        market_rule_id: i32,
    },
    ReqPnl {
        req_id: i32,
        account: String,
        model_code: String,
    },
    CancelPnl {
        req_id: i32,
    },
    ReqPnlSingle {
        req_id: i32,
        account: String,
        model_code: String,
        con_id: i32,
    },
    CancelPnlSingle {
        req_id: i32,
    },
    ReqHistoricalTicks {
        req_id: i32,
        contract: ContractPreamble,
        trading_class: String,
        include_expired: bool,
        start_date_time: String,
        end_date_time: String,
        number_of_ticks: i32,
        what_to_show: String,
        use_rth: i32,
        ignore_size: bool,
        misc_options: String,
    },
    ReqTickByTickData {
        req_id: i32,
        contract: ContractPreamble,
        trading_class: String,
        tick_type: String,
        number_of_ticks: i32,
        ignore_size: bool,
    },
    CancelTickByTickData {
        req_id: i32,
    },
    ReqCompletedOrders {
        api_only: bool,
    },
}

//==================================================================================================
pub fn make_message(msg: &str) -> Result<Vec<u8>, IBKRApiLibError> {
    //let mut buffer = ByteBuffer::new();
    let mut buffer: Vec<u8> = Vec::new();

    buffer.extend_from_slice(&i32::to_be_bytes(msg.len() as i32));

    buffer.write(msg.as_ascii_str().unwrap().as_bytes())?;
    let tmp = buffer.clone();
    //debug!("Message after create: {:?}", buffer);

    let (_size, _msg, _buf) = read_msg(tmp.as_slice())?;
    //debug!("Message read: size:{}, msg:{}, bytes: {:?}", size, msg, buf);

    Ok(tmp)
}

//==================================================================================================
pub fn read_msg<'a>(buf: &[u8]) -> Result<(usize, String, Vec<u8>), IBKRApiLibError> {
    // first the size prefix and then the corresponding msg payload ""

    if buf.len() < 4 {
        debug!("read_msg:  buffer too small!! {:?}", buf.len());
        return Ok((0, String::new(), buf.to_vec()));
    }

    let size = i32::from_be_bytes(buf[0..4].try_into().unwrap()) as usize;
    //debug!("read_msg: Message size: {:?}", size);

    if buf.len() - 4 >= size {
        let text = String::from_utf8(buf[4..4 + size].to_vec()).unwrap();
        //debug!("read_msg: text in read message: {:?}", text);
        Ok((size, text, buf[4 + size..].to_vec()))
    } else {
        Ok((size, String::new(), buf.to_vec()))
    }
}

//==================================================================================================
pub fn read_fields(buf: &str) -> Vec<String> {
    //msg payload is made of fields terminated/separated by NULL chars
    let a = '\u{0}';
    let mut fields: Vec<&str> = buf.split(a).collect::<Vec<&str>>();
    //debug!("fields.len() in read_fields: {}", fields.len());
    //last one is empty
    fields.remove(fields.len() - 1);

    fields
        .iter()
        .map(|x| String::from(*x))
        .collect::<Vec<String>>()
}

//==================================================================================================
pub fn make_field(val: &dyn Any) -> Result<String, IBKRApiLibError> {
    // debug!("CALLING make_field!!");
    // adds the NULL string terminator
    let mut field = "\0".to_string();
    // bool type is encoded as int
    if let Some(boolval) = val.downcast_ref::<bool>() {
        field = format!("{}\0", *boolval as i32);
    } else if let Some(stringval) = val.downcast_ref::<usize>() {
        field = format!("{}\0", *stringval as i32);
    } else if let Some(stringval) = val.downcast_ref::<f64>() {
        if UNSET_DOUBLE == *stringval {
            field = format!("{}\0", "");
        } else {
            field = format!("{}\0", *stringval as f64);
        }
    } else if let Some(stringval) = val.downcast_ref::<i32>() {
        if UNSET_INTEGER == *stringval {
            field = format!("{}\0", "");
        } else {
            field = format!("{}\0", *stringval as i32);
        }
    } else if let Some(stringval) = val.downcast_ref::<String>() {
        field = format!("{}\0", stringval);
    } else if let Some(stringval) = val.downcast_ref::<&str>() {
        field = format!("{}\0", stringval);
    }

    Ok(field)
}

//==================================================================================================
pub fn make_field_handle_empty(val: &dyn Any) -> Result<String, IBKRApiLibError> {
    if let Some(stringval) = val.downcast_ref::<f64>() {
        if UNSET_DOUBLE == *stringval {
            return make_field(&"");
        }
    } else if let Some(stringval) = val.downcast_ref::<i32>() {
        if UNSET_INTEGER == *stringval {
            return make_field(&"");
        }
    }

    make_field(val)
}
